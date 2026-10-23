use super::archive::{Entry, MAGIC, Manifest, pack_part};
use crate::error::{Result, failure};
use crate::model::{Message, Project};
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

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

// Borrow metadata through compression rather than cloning the project and
// constructing a second JSON tree for annotations, messages, and imported data.
enum Metadata<'a> {
    Imported(&'a Value),
    Project(&'a Project),
    Messages(&'a [Message]),
    Upstream(&'a Project, Option<&'a serde_json::Map<String, Value>>),
}

impl Serialize for Metadata<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Imported(value) => value.serialize(serializer),
            Self::Messages(messages) => messages.serialize(serializer),
            Self::Project(project) => {
                let mut map = serializer.serialize_map(Some(7))?;
                map.serialize_entry("bookmarks", &project.bookmarks)?;
                map.serialize_entry("imported", &BTreeMap::<String, Value>::new())?;
                map.serialize_entry("notes", &project.notes)?;
                map.serialize_entry("patches", &project.patches)?;
                map.serialize_entry("renamed", &project.renamed)?;
                map.serialize_entry("source_document", &project.source_document)?;
                map.serialize_entry("target", &project.target)?;
                map.end()
            }
            Self::Upstream(project, original) => {
                let mut map = serializer.serialize_map(None)?;
                if let Some(original) = original {
                    for (key, value) in *original {
                        if key != "names" && key != "notes" {
                            map.serialize_entry(key, value)?;
                        }
                    }
                }
                map.serialize_entry("names", &project.renamed)?;
                map.serialize_entry("notes", &project.notes)?;
                map.end()
            }
        }
    }
}

fn metadata<'a>(
    project: &'a Project,
    messages: &'a [Message],
) -> Result<BTreeMap<&'a str, Metadata<'a>>> {
    let mut parts: BTreeMap<_, _> = project
        .imported
        .iter()
        .map(|(name, value)| (name.as_str(), Metadata::Imported(value)))
        .collect();
    parts.insert("rust-project.json", Metadata::Project(project));
    parts.insert("rust-chat.json", Metadata::Messages(messages));
    let original = project
        .imported
        .get("project.json")
        .map(|value| {
            value
                .as_object()
                .ok_or("The imported project.json metadata is not an object")
        })
        .transpose()?;
    parts.insert("project.json", Metadata::Upstream(project, original));
    if parts.len() >= 256 {
        return Err(failure("The document has too many metadata entries"));
    }
    Ok(parts)
}

fn pack_payload<T: Serialize>(
    project: &Project,
    parts: BTreeMap<&str, T>,
    payload: &mut File,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut offset = 0;
    let mut metadata_bytes = 0;
    for (name, value) in parts {
        let entry = super::archive_json::pack(
            name,
            &value,
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
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64,
        app: format!("Prism {}", env!("CARGO_PKG_VERSION")),
        entries,
    };
    let manifest = serde_json::to_vec(&manifest)?;
    if manifest.len() > 2 * 1024 * 1024 {
        return Err(failure("The document table of contents is too large"));
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests;
