use crate::error::{Result, failure};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission<'a> {
    pub lease: String,
    #[serde(borrow)]
    pub files: Vec<SourceFile<'a>>,
    pub results: Vec<UnitResult>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceFile<'a> {
    pub path: String,
    #[serde(borrow)]
    pub content: &'a str,
    pub previous_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnitResult {
    pub address: String,
    pub state: String,
    pub reason: String,
    pub paths: Vec<String>,
}

impl Submission<'_> {
    pub fn validate(&self) -> Result<()> {
        if self.lease.len() != 32 || !self.lease.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(failure("Supply a valid batch lease token"));
        }
        if self.results.is_empty() || self.results.len() > 64 || self.files.len() > 64 {
            return Err(failure("Submit 1–64 unit results and at most 64 files"));
        }
        let mut paths = BTreeSet::new();
        let mut total = 0;
        for file in &self.files {
            super::objects::validate_path(&file.path)?;
            if !file.previous_sha256.is_empty()
                && (file.previous_sha256.len() != 64
                    || !file
                        .previous_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()))
            {
                return Err(failure(
                    "Use the file's previous SHA-256, or an empty string for a new file",
                ));
            }
            total += file.content.len();
            if file.content.len() > super::objects::FILE_LIMIT || !paths.insert(&file.path) {
                return Err(failure(
                    "Files must have unique paths and be at most 1 MiB each",
                ));
            }
        }
        if total > 4 * 1024 * 1024 {
            return Err(failure("A source batch is limited to 4 MiB"));
        }
        let mut addresses = BTreeSet::new();
        for result in &self.results {
            if result.address.len() > 1024
                || !addresses.insert(&result.address)
                || result.reason.len() > 4096
                || result.paths.len() > 64
            {
                return Err(failure(
                    "Use unique units, at most 64 paths and a reason up to 4096 bytes",
                ));
            }
            if (result.state == "generated" && result.paths.is_empty())
                || (result.state == "blocked" && result.reason.trim().is_empty())
                || !matches!(result.state.as_str(), "generated" | "blocked")
            {
                return Err(failure(
                    "Generated units need source paths; blocked units need a reason",
                ));
            }
            for path in &result.paths {
                super::objects::validate_path(path)?;
            }
        }
        Ok(())
    }
}
