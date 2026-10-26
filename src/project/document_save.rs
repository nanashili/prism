use super::archive::{Entry, MAGIC, Manifest, pack_part};
use crate::error::{Result, failure};
use crate::model::{Message, Project};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn save(path: &Path, project: &Project, messages: &[Message]) -> Result<()> {
    super::files::preserve_original(&project.target.path, path)?;
    let parent = path
        .parent()
        .ok_or("The destination has no parent folder")?;
    std::fs::create_dir_all(parent)?;
    let mut payload = tempfile::tempfile_in(parent)?;
    let parts = metadata(project, messages)?;
    let entries = pack_payload(project, parts, &mut payload)?;
    let manifest = manifest(project, entries)?;
    payload.seek(SeekFrom::Start(0))?;

    crate::storage::atomic::write(path, |file| {
        file.write_all(MAGIC)?;
        file.write_all(&u32::try_from(manifest.len())?.to_le_bytes())?;
        file.write_all(&manifest)?;
        std::io::copy(&mut payload, file)?;
        Ok(())
    })
}

fn metadata<'a>(
    project: &'a Project,
    messages: &[Message],
) -> Result<BTreeMap<&'a str, Cow<'a, Value>>> {
    let mut parts: BTreeMap<_, _> = project
        .imported
        .iter()
        .map(|(name, value)| (name.as_str(), Cow::Borrowed(value)))
        .collect();
    let mut state = project.clone();
    state.imported = Default::default();
    parts.insert(
        "rust-project.json",
        Cow::Owned(serde_json::to_value(&state)?),
    );
    parts.insert(
        "rust-chat.json",
        Cow::Owned(serde_json::to_value(messages)?),
    );
    let mut upstream = parts
        .remove("project.json")
        .unwrap_or_else(|| Cow::Owned(json!({})));
    let properties = upstream
        .to_mut()
        .as_object_mut()
        .ok_or("The imported project.json metadata is not an object")?;
    properties.insert("names".into(), serde_json::to_value(&project.renamed)?);
    properties.insert("notes".into(), serde_json::to_value(&project.notes)?);
    parts.insert("project.json", upstream);
    if parts.len() >= 256 {
        return Err(failure("The document has too many metadata entries"));
    }
    Ok(parts)
}

fn pack_payload(
    project: &Project,
    parts: BTreeMap<&str, Cow<'_, Value>>,
    payload: &mut File,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut offset = 0;
    let mut metadata_bytes = 0;
    for (name, value) in parts {
        let entry = super::archive_json::pack(
            name,
            value.as_ref(),
            payload,
            offset,
            32 * 1024 * 1024 - metadata_bytes,
        )?;
        metadata_bytes += entry.raw;
        offset += entry.size;
        entries.push(entry);
    }
    let binary = pack_part("binary", File::open(&project.target.path)?, payload, offset)?;
    if binary.sha256 != project.target.sha256 {
        return Err(failure(
            "The input changed. Reopen it before saving a project.",
        ));
    }
    entries.push(binary);
    Ok(entries)
}

fn manifest(project: &Project, entries: Vec<Entry>) -> Result<Vec<u8>> {
    let manifest = Manifest {
        format: 1,
        name: project.target.name.clone(),
        kind: project.target.kind.clone(),
        arch: project.target.arch.clone(),
        meta: format!("{} bytes", project.target.size),
        binary: project.target.sha256.clone(),
        saved_at: u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?,
        app: format!("Prism {}", env!("CARGO_PKG_VERSION")),
        entries,
    };
    let manifest = serde_json::to_vec(&manifest)?;
    if manifest.len() > 2 * 1024 * 1024 {
        return Err(failure("The document table of contents is too large"));
    }
    Ok(manifest)
}
