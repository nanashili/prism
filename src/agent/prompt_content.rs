use super::Prompt;
use crate::error::{Result, failure};
use crate::model::Attachment;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::fs::File;
use std::io::Read;

// Base64 expansion must leave room for text in the 8 MiB protocol frame.
const IMAGE_BUDGET: usize = 4 * 1024 * 1024;

pub(super) fn build(prompt: &Prompt, history: &str, images: bool) -> Result<Vec<Value>> {
    // Codex receives reconnect history through its session-instructions extension.
    let history = if super::codex::selected(prompt) {
        ""
    } else {
        history
    };
    // Adapters parse slash commands at the start of the prompt. Context must
    // never turn /goal, /compact or another advertised command into ordinary text.
    let text = if prompt.text.trim_start().starts_with('/') {
        prompt.text.trim_start().to_owned()
    } else {
        format!("{history}\n{}\n\n{}", prompt.context, prompt.text)
    };
    let mut content = vec![json!({"type":"text", "text":text})];
    let mut remaining = IMAGE_BUDGET;
    for attachment in &prompt.attachments {
        let block = attachment_content(attachment, images, &mut remaining)
            .map_err(|error| failure(format!("Couldn't attach {}: {error}", attachment.name)))?;
        content.push(block);
    }
    Ok(content)
}

fn attachment_content(
    attachment: &Attachment,
    images: bool,
    remaining: &mut usize,
) -> Result<Value> {
    let mut link = json!({
        "type":"resource_link", "name":attachment.name, "uri":attachment.uri
    });
    let Some(path) = attachment.file_path() else {
        return Ok(link);
    };
    let metadata = path.metadata()?;
    if !metadata.is_file() {
        return Ok(link);
    }
    link["size"] = metadata.len().into();
    if images
        && metadata.len() <= *remaining as u64
        && *remaining > 0
        && let Some(image) = image_content(&path, attachment, remaining)?
    {
        return Ok(image);
    }
    Ok(link)
}

fn image_content(
    path: &std::path::Path,
    attachment: &Attachment,
    remaining: &mut usize,
) -> Result<Option<Value>> {
    let mut file = File::open(path)?;
    let mut header = [0; 16];
    let length = file.read(&mut header)?;
    let mime = match image::guess_format(&header[..length]) {
        Ok(image::ImageFormat::Png) => "image/png",
        Ok(image::ImageFormat::Jpeg) => "image/jpeg",
        Ok(image::ImageFormat::Gif) => "image/gif",
        Ok(image::ImageFormat::WebP) => "image/webp",
        _ => return Ok(None),
    };

    let mut bytes = header[..length].to_vec();
    let capacity = remaining.saturating_sub(length);
    file.take(capacity as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > *remaining {
        // The file grew after metadata was read. Keep the bounded path reference.
        return Ok(None);
    }
    *remaining -= bytes.len();
    Ok(Some(json!({
        "type":"image", "mimeType":mime, "uri":attachment.uri,
        "data":STANDARD.encode(&bytes)
    })))
}
