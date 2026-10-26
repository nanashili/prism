use crate::error::{Result, failure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Take, Write};
use std::path::Path;

pub(crate) const MAGIC: &[u8; 8] = b"PRISM\0\0\x01";

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub name: String,
    pub codec: String,
    pub offset: u64,
    pub size: u64,
    pub raw: u64,
    pub sha256: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Manifest {
    pub format: u32,
    pub name: String,
    pub kind: String,
    pub arch: String,
    pub meta: String,
    pub binary: String,
    pub saved_at: u64,
    pub app: String,
    pub entries: Vec<Entry>,
}

pub(crate) struct Archive {
    file: File,
    base: u64,
    pub manifest: Manifest,
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 12];
        file.read_exact(&mut header)?;
        if &header[..8] != MAGIC {
            return Err(failure("This is not a Prism document"));
        }
        let length = u32::from_le_bytes(header[8..12].try_into()?) as usize;
        if length > 2 * 1024 * 1024 {
            return Err(failure("The document table of contents is too large"));
        }
        let mut json = vec![0; length];
        file.read_exact(&mut json)?;
        let manifest: Manifest = serde_json::from_slice(&json)?;
        let base = 12 + length as u64;
        validate(&manifest, base, file.metadata()?.len())?;
        Ok(Self {
            file,
            base,
            manifest,
        })
    }

    fn source(&mut self, name: &str, limit: u64) -> Result<(Box<dyn Read + '_>, Entry)> {
        let entry = self
            .manifest
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| failure(format!("The document has no {name} entry")))?
            .clone();
        if entry.raw > limit {
            return Err(failure(format!(
                "The {name} entry exceeds the configured read limit"
            )));
        }
        self.file.seek(SeekFrom::Start(self.base + entry.offset))?;
        let source = (&mut self.file).take(entry.size);
        let reader: Box<dyn Read + '_> = match entry.codec.as_str() {
            "zstd" => {
                let mut decoder = zstd::stream::read::Decoder::new(source)?;
                decoder.window_log_max(25)?;
                Box::new(decoder)
            }
            "none" => Box::new(source),
            _ => {
                return Err(failure(
                    "This document uses an unsupported compression codec",
                ));
            }
        };
        Ok((reader, entry))
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn extract(&mut self, name: &str, output: &mut impl Write, limit: u64) -> Result<()> {
        let (reader, entry) = self.source(name, limit)?;
        let mut reader = VerifiedReader::new(reader, entry)?;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                return Ok(());
            }
            output.write_all(&buffer[..count])?;
        }
    }

    pub fn json(&mut self, name: &str) -> Result<serde_json::Value> {
        let (reader, entry) = self.source(name, 32 * 1024 * 1024)?;
        let mut reader = BufReader::with_capacity(64 * 1024, VerifiedReader::new(reader, entry)?);
        let value = serde_json::from_reader(&mut reader);
        // Even malformed JSON must pass the original size/digest checks first.
        // Drain the bounded entry so parse failures cannot hide corrupt trailing bytes.
        std::io::copy(&mut reader, &mut std::io::sink())?;
        Ok(value?)
    }
}

fn validate(manifest: &Manifest, base: u64, size: u64) -> Result<()> {
    if manifest.format != 1 || manifest.entries.len() > 256 {
        return Err(failure("Unsupported Prism document version or entry count"));
    }
    let mut names = BTreeSet::new();
    for entry in &manifest.entries {
        let end = base
            .checked_add(entry.offset)
            .and_then(|offset| offset.checked_add(entry.size));
        if end.is_none_or(|end| end > size) || !names.insert(&entry.name) {
            return Err(failure(
                "The Prism document has damaged or duplicate entries",
            ));
        }
    }
    Ok(())
}

struct VerifiedReader<R> {
    source: Take<R>,
    entry: Entry,
    digest: Sha256,
    total: u64,
}

impl<R: Read> VerifiedReader<R> {
    fn new(source: R, entry: Entry) -> Result<Self> {
        let limit = entry.raw.checked_add(1).ok_or("Invalid decoded size")?;
        Ok(Self {
            source: source.take(limit),
            entry,
            digest: Sha256::new(),
            total: 0,
        })
    }
}

impl<R: Read> Read for VerifiedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let count = self.source.read(buffer)?;
        self.total += count as u64;
        if self.total > self.entry.raw {
            return Err(std::io::Error::other(
                "The document entry expanded beyond its recorded size",
            ));
        }
        self.digest.update(&buffer[..count]);
        if count == 0
            && (self.total != self.entry.raw
                || format!("{:x}", self.digest.clone().finalize()) != self.entry.sha256)
        {
            return Err(std::io::Error::other(format!(
                "The document entry {} failed its integrity check",
                self.entry.name
            )));
        }
        Ok(count)
    }
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn decode(bytes: &[u8], limit: u64) -> Result<Vec<u8>> {
    let mut decoder = zstd::stream::read::Decoder::new(bytes)?;
    decoder.window_log_max(25)?;
    let mut output = Vec::new();
    decoder
        .take(limit.saturating_add(1))
        .read_to_end(&mut output)?;
    if output.len() as u64 > limit {
        return Err(failure("Compressed data exceeds its read limit"));
    }
    Ok(output)
}

pub(crate) fn pack_part(
    name: &str,
    reader: impl Read,
    output: &mut File,
    offset: u64,
) -> Result<Entry> {
    let start = output.stream_position()?;
    let mut encoder = zstd::stream::write::Encoder::new(&mut *output, 3)?;
    let mut digest = Sha256::new();
    let mut reader = reader;
    let mut buffer = [0u8; 64 * 1024];
    let mut raw = 0;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        encoder.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
        raw += count as u64;
    }
    encoder.finish()?;
    Ok(Entry {
        name: name.into(),
        codec: "zstd".into(),
        offset,
        size: output.stream_position()? - start,
        raw,
        sha256: format!("{:x}", digest.finalize()),
    })
}

#[cfg(test)]
mod tests;
