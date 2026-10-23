use crate::error::{Result, failure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use url::Url;

/// A reference only: file contents are loaded by the agent worker when needed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Attachment {
    pub name: String,
    pub uri: String,
}

impl Attachment {
    pub const LIMIT: usize = 32;
    pub const LOCATION_LIMIT: usize = 4096;

    pub fn from_path(path: &Path) -> Result<Self> {
        let uri = Url::from_file_path(path)
            .map_err(|_| failure("Choose an absolute file or folder path"))?;
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy();
        Self::new(&name, uri)
    }

    pub fn from_location(location: &str) -> Result<Self> {
        let location = location.trim();
        if location.is_empty() || location.len() > Self::LOCATION_LIMIT {
            return Err(failure("Enter a path or URL up to 4096 bytes long"));
        }
        if let Some(relative) = location.strip_prefix("~/") {
            let home = dirs::home_dir().ok_or("The home folder is unavailable")?;
            return Self::from_path(&home.join(relative));
        }
        if Path::new(location).is_absolute() {
            return Self::from_path(Path::new(location));
        }

        let uri = Url::parse(location)?;
        if uri.scheme() == "file" {
            let path = uri
                .to_file_path()
                .map_err(|_| failure("Invalid local file URL"))?;
            return Self::from_path(&path);
        }
        if !matches!(uri.scheme(), "https" | "http") || uri.host_str().is_none() {
            return Err(failure("Use an absolute path, file URL or http(s) URL"));
        }
        let name: String = uri.as_str().chars().take(128).collect();
        Self::new(&name, uri)
    }

    fn new(name: &str, uri: Url) -> Result<Self> {
        if uri.as_str().len() > Self::LOCATION_LIMIT {
            return Err(failure("This attachment location is too long"));
        }
        Ok(Self {
            name: name.chars().take(128).collect(),
            uri: uri.into(),
        })
    }

    pub fn file_path(&self) -> Option<PathBuf> {
        Url::parse(&self.uri).ok()?.to_file_path().ok()
    }

    pub fn message_text(draft: &str, attachments: &[Self]) -> String {
        let mut text = draft.trim().to_owned();
        if attachments.is_empty() {
            return text;
        }
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str("Attachments:\n");
        for attachment in attachments {
            let name = attachment
                .name
                .replace('\\', "\\\\")
                .replace('[', "\\[")
                .replace(']', "\\]")
                .replace(['\r', '\n'], " ");
            text.push_str(&format!("- [{name}](<{}>)\n", attachment.uri));
        }
        text
    }
}
