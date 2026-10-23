use super::archive::Entry;
use crate::error::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufWriter, Seek, Write};

pub(super) fn pack(
    name: &str,
    value: &impl Serialize,
    output: &mut File,
    offset: u64,
    limit: u64,
) -> Result<Entry> {
    let start = output.stream_position()?;
    let encoder = zstd::stream::write::Encoder::new(&mut *output, 3)?;
    let mut writer = MetadataWriter {
        encoder,
        digest: Sha256::new(),
        raw: 0,
        limit,
    };
    {
        let mut buffered = BufWriter::with_capacity(64 * 1024, &mut writer);
        serde_json::to_writer(&mut buffered, value)?;
        buffered.flush()?;
    }
    let MetadataWriter {
        encoder,
        digest,
        raw,
        ..
    } = writer;
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

struct MetadataWriter<W: Write> {
    encoder: W,
    digest: Sha256,
    raw: u64,
    limit: u64,
}

impl<W: Write> Write for MetadataWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.limit - self.raw {
            return Err(std::io::Error::other(
                "The document metadata exceeds 32 MiB",
            ));
        }
        let count = self.encoder.write(bytes)?;
        self.digest.update(&bytes[..count]);
        self.raw += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.encoder.flush()
    }
}
