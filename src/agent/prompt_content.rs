use super::{Prompt, PromptContext};
use crate::error::{Result, failure};
use crate::model::Attachment;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::fs::File;
use std::io::Read;

// Base64 expansion must leave room for text in the 8 MiB protocol frame.
const IMAGE_BUDGET: usize = 4 * 1024 * 1024;

pub(super) fn build(prompt: &Prompt, history: &str, images: ImageSupport) -> Result<Vec<Value>> {
    let images = images == ImageSupport::Enabled;

    // Codex receives reconnect history through its session-instructions extension.
    let history = if super::codex::selected(prompt) {
        ""
    } else {
        history
    };
    // Adapters parse slash commands at the start of the prompt. Context must
    // never turn /goal, /compact or another advertised command into ordinary text.
    let text = render_text(prompt, history);
    let mut content = vec![json!({"type":"text", "text":text})];
    let mut remaining = IMAGE_BUDGET;
    for attachment in &prompt.attachments {
        let block = attachment_content(attachment, images, &mut remaining)
            .map_err(|error| failure(format!("Couldn't attach {}: {error}", attachment.name)))?;
        content.push(block);
    }
    Ok(content)
}

fn render_text(prompt: &Prompt, history: &str) -> String {
    if prompt.text.trim_start().starts_with('/') {
        return prompt.text.trim_start().to_owned();
    }

    let context = if super::codex::selected(prompt) {
        &prompt.context.dynamic_context
    } else {
        &prompt.context.combined()
    };
    format!("{history}\n{context}\n\n{}", prompt.text)
}

impl PromptContext {
    fn combined(&self) -> String {
        format!("{}\n{}", self.static_instructions, self.dynamic_context)
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImageSupport {
    Enabled,
    Disabled,
}

impl From<bool> for ImageSupport {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageSupport, build};
    use crate::agent::Prompt;
    use crate::agent::PromptContext;
    use crate::settings::Settings;
    use std::path::PathBuf;

    fn prompt(context: PromptContext, text: &str, codex: bool) -> Prompt {
        let settings = Settings {
            agent: if codex { "codex-acp" } else { "other-agent" }.into(),
            ..Settings::default()
        };
        Prompt {
            chat: 1,
            project: None,
            file: None,
            text: text.into(),
            attachments: Vec::new(),
            context,
            history: String::new(),
            cwd: PathBuf::new(),
            settings,
            servers: Vec::new(),
            picks: Vec::new(),
        }
    }

    #[test]
    fn slash_commands_are_sent_without_context() {
        let prompt = prompt(
            PromptContext {
                static_instructions: "static".into(),
                dynamic_context: "dynamic".into(),
            },
            "  /compact",
            false,
        );

        let content = build(&prompt, "history", ImageSupport::Disabled)
            .expect("Slash command fixture has no fallible attachments");

        assert_eq!(content[0]["text"], "/compact");
    }

    #[test]
    fn non_codex_prompts_keep_history_and_both_context_layers() {
        let prompt = prompt(
            PromptContext {
                static_instructions: "static".into(),
                dynamic_context: "dynamic".into(),
            },
            "question",
            false,
        );

        let content = build(&prompt, "history", ImageSupport::Disabled)
            .expect("Text prompt fixture has no fallible attachments");

        assert_eq!(content[0]["text"], "history\nstatic\ndynamic\n\nquestion");
    }

    #[test]
    fn codex_session_prompt_keeps_dynamic_context_without_static_instructions() {
        let prompt = prompt(
            PromptContext {
                static_instructions: "static".into(),
                dynamic_context: "dynamic".into(),
            },
            "question",
            true,
        );

        let content = build(&prompt, "history", ImageSupport::Disabled)
            .expect("Codex prompt fixture has no fallible attachments");

        assert_eq!(content[0]["text"], "\ndynamic\n\nquestion");
        assert!(
            !content[0]["text"]
                .as_str()
                .expect("The first prompt block is text")
                .contains("static")
        );
    }

    #[test]
    fn codex_follow_up_uses_current_dynamic_context_without_repeating_history() {
        let mut prompt = prompt(
            PromptContext {
                static_instructions: "static".into(),
                dynamic_context: "first view".into(),
            },
            "first question",
            true,
        );

        let first = build(&prompt, "old history", ImageSupport::Disabled)
            .expect("First prompt fixture has no fallible attachments");
        assert_eq!(first[0]["text"], "\nfirst view\n\nfirst question");

        prompt.context.dynamic_context = "second view".into();
        prompt.text = "follow-up question".into();
        let follow_up = build(
            &prompt,
            "old history\nfirst question",
            ImageSupport::Disabled,
        )
        .expect("Follow-up fixture has no fallible attachments");

        assert_eq!(follow_up[0]["text"], "\nsecond view\n\nfollow-up question");
    }
}
