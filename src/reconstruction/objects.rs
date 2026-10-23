use crate::error::{Result, failure};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Component, Path};

pub(super) const FILE_LIMIT: usize = 1024 * 1024;

pub(super) fn put(db: &Connection, bytes: &[u8]) -> Result<String> {
    if bytes.len() > FILE_LIMIT {
        return Err(failure(
            "Reconstruction files and evidence are limited to 1 MiB each",
        ));
    }
    let digest = digest(bytes);
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM objects WHERE digest=?1)",
        [&digest],
        |row| row.get(0),
    )?;
    if !exists {
        let compressed = zstd::stream::encode_all(bytes, 3)?;
        db.execute(
            "INSERT INTO objects VALUES(?1,?2,?3)",
            params![digest, i64::try_from(bytes.len())?, compressed],
        )?;
    }
    Ok(digest)
}

pub(super) fn get(db: &Connection, digest: &str) -> Result<Vec<u8>> {
    let (size, compressed): (i64, Vec<u8>) = db.query_row(
        "SELECT size,substr(data,1,1052673) FROM objects WHERE digest=?1",
        [digest],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if size < 0 || size > FILE_LIMIT as i64 || compressed.len() > FILE_LIMIT + 4096 {
        return Err(failure(
            "Stored reconstruction object exceeds its size limit",
        ));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    zstd::stream::read::Decoder::new(compressed.as_slice())?
        .take(FILE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as i64 != size || self::digest(&bytes) != digest {
        return Err(failure(
            "Stored reconstruction object failed its integrity check",
        ));
    }
    Ok(bytes)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn digest_json(value: &serde_json::Value) -> Result<String> {
    let mut writer = DigestWriter(Sha256::new());
    serde_json::to_writer(&mut writer, value)?;
    Ok(format!("{:x}", writer.0.finalize()))
}

struct DigestWriter(Sha256);

impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn validate_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.len() > 1024
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || path
            .split('/')
            .any(|part| part == ".prism-reconstruction" || part == ".git")
    {
        return Err(failure(
            "Use a relative source path without parent traversal or metadata folders",
        ));
    }
    Ok(())
}
