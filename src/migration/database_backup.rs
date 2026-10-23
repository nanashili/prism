use crate::error::{Result, failure};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

pub(super) fn compress(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let expected = checksum(File::open(path)?)?;
    let destination = path.with_extension("sqlite.zst");
    crate::storage::atomic::write(&destination, |output| {
        let mut encoder = zstd::stream::write::Encoder::new(&mut *output, 3)?;
        std::io::copy(&mut File::open(path)?, &mut encoder)?;
        encoder.finish()?;

        output.rewind()?;
        let mut decoder = zstd::stream::read::Decoder::new(output)?;
        decoder.window_log_max(25)?;
        if checksum(decoder)? != expected {
            return Err(failure(
                "The compressed reconstruction backup failed verification",
            ));
        }
        Ok(())
    })?;
    // Delete the uncompressed snapshot only after verifying the committed replacement.
    std::fs::remove_file(path)?;
    Ok(())
}

fn checksum(mut reader: impl Read) -> Result<(u64, [u8; 32])> {
    let mut digest = Sha256::new();
    let mut size = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size += count as u64;
    }
    Ok((size, digest.finalize().into()))
}
