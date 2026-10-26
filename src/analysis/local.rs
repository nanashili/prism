use crate::error::{Result, failure};
use crate::project::files;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn strings(path: &Path, cancelled: &AtomicBool) -> Result<String> {
    use std::fmt::Write;
    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; 64 * 1024];
    let mut found = String::new();
    let mut current = String::new();
    let mut offset = 0u64;
    let mut start = 0u64;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        for byte in &buffer[..count] {
            if (32..127).contains(byte) {
                if current.is_empty() {
                    start = offset;
                }
                if current.len() < 2048 {
                    current.push(char::from(*byte));
                }
            } else {
                if current.len() >= 4 {
                    writeln!(found, "{start:012x}  {current}")?;
                }
                current.clear();
            }
            offset += 1;
            if found.len() >= files::TEXT_LIMIT {
                return Ok(found);
            }
        }
    }
    if current.len() >= 4 {
        writeln!(found, "{start:012x}  {current}")?;
    }
    Ok(found)
}

pub(crate) fn model(path: &Path) -> Result<String> {
    let header = files::bytes(path, 0, 4096)?;
    if header.starts_with(b"GGUF") && header.len() >= 24 {
        let version = u32::from_le_bytes(header[4..8].try_into()?);
        let tensors = u64::from_le_bytes(header[8..16].try_into()?);
        let metadata = u64::from_le_bytes(header[16..24].try_into()?);
        return Ok(format!(
            "GGUF version {version}\nTensors: {tensors}\nMetadata pairs: {metadata}\n\n{}",
            files::hex(path, 0)?
        ));
    }
    if path
        .extension()
        .is_some_and(|extension| extension == "safetensors")
        && header.len() >= 8
    {
        let length = u64::from_le_bytes(header[..8].try_into()?);
        if length > files::TEXT_LIMIT as u64 {
            return Err(failure(
                "The tensor metadata exceeds the 1 MiB display limit",
            ));
        }
        let data = files::bytes(path, 8, usize::try_from(length)?)?;
        let metadata: serde_json::Value = serde_json::from_slice(&data)?;
        return Ok(serde_json::to_string_pretty(&metadata)?);
    }
    Ok(format!(
        "Model header (data is never executed)\n\n{}",
        files::hex(path, 0)?
    ))
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn resource_list(path: &Path, cancelled: &AtomicBool) -> Result<String> {
    let root = path
        .parent()
        .and_then(Path::parent)
        .filter(|root| root.file_name().is_some_and(|name| name == "Contents"))
        .map_or_else(
            || path.parent().unwrap_or(path).to_path_buf(),
            |root| root.join("Resources"),
        );
    let mut pending = vec![root.clone()];
    let mut result = String::new();
    let mut count = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory)? {
            if cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            let entry = entry?;
            let kind = entry.file_type()?;
            count += 1;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                result.push_str(&entry.path().strip_prefix(&root)?.to_string_lossy());
                result.push('\n');
            }
            if count >= 10_000 || result.len() >= files::TEXT_LIMIT {
                result.push_str("\nResource listing reached its display limit.\n");
                return Ok(result);
            }
        }
    }
    Ok(result)
}
