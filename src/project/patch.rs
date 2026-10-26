use crate::error::{Result, failure};
use crate::model::Project;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

pub(crate) fn parse_hex(text: &str) -> Result<Vec<u8>> {
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if compact.is_empty() || !compact.len().is_multiple_of(2) || compact.len() > 8192 {
        return Err(failure(
            "Enter between 1 and 4096 bytes as pairs of hexadecimal digits",
        ));
    }
    compact
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(text, 16)?)
        })
        .collect()
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn export(project: &Project, destination: &Path) -> Result<()> {
    super::files::preserve_original(&project.target.path, destination)?;
    for (offset, bytes) in &project.patches {
        if offset
            .checked_add(bytes.len() as u64)
            .is_none_or(|end| end > project.target.size)
        {
            return Err(failure("A patch extends beyond the input file"));
        }
    }
    crate::storage::atomic::write(destination, |output| {
        copy_input(project, output)?;
        for (offset, bytes) in &project.patches {
            output.seek(SeekFrom::Start(*offset))?;
            output.write_all(bytes)?;
        }
        Ok(())
    })
}

fn copy_input(project: &Project, output: &mut File) -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut input = File::open(&project.target.path)?;
    let permissions = input.metadata()?.permissions();
    let mut digest = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
    }
    if format!("{:x}", digest.finalize()) != project.target.sha256 {
        return Err(failure(
            "The original file changed. Reopen it before applying patches.",
        ));
    }
    output.set_permissions(permissions)?;
    Ok(())
}
