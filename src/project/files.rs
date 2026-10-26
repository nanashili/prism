use crate::error::{Result, failure};
use crate::model::Target;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) const TEXT_LIMIT: usize = 1024 * 1024;

pub(crate) fn preserve_original(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() && std::fs::canonicalize(destination)? == std::fs::canonicalize(source)?
    {
        return Err(failure(
            "Choose a different destination to preserve the original file",
        ));
    }
    Ok(())
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn hash(path: &Path, cancelled: &AtomicBool) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(crate) fn executable(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        return Ok(path.to_path_buf());
    }
    let binaries = path.join("Contents/MacOS");
    let entries = std::fs::read_dir(binaries)?;
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            candidates.push(entry.path());
        }
    }
    candidates.sort();
    let name = path.file_stem();
    candidates
        .iter()
        .find(|file| file.file_name() == name)
        .cloned()
        .or_else(|| candidates.into_iter().next())
        .ok_or_else(|| failure("This app bundle has no executable in Contents/MacOS"))
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn inspect(path: &Path, cancelled: &AtomicBool) -> Result<Target> {
    let actual = executable(path)?;
    let header = bytes(&actual, 0, 32)?;
    let (kind, arch) = identify(&actual, &header);
    Ok(Target {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        sha256: hash(&actual, cancelled)?,
        size: std::fs::metadata(&actual)?.len(),
        original_path: Some(path.to_path_buf()),
        path: actual,
        kind: if path.is_dir() { "app".into() } else { kind },
        arch,
    })
}

fn identify(path: &Path, header: &[u8]) -> (String, String) {
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let kind = match extension.as_str() {
        "apk" | "xapk" | "apks" | "dex" => "android",
        "gguf" | "safetensors" | "onnx" | "pt" | "pth" => "model",
        "metallib" | "ptx" | "cubin" | "fatbin" | "hsaco" | "spv" => "gpu",
        "rs" | "c" | "cpp" | "h" | "py" | "swift" | "js" | "ts" | "cs" | "java" | "metal" => {
            "source"
        }
        _ => "binary",
    };
    let arch = if header.starts_with(b"\x7fELF") {
        "ELF"
    } else if header.starts_with(b"MZ") {
        "PE"
    } else if header.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]) {
        if header.get(4) == Some(&12) {
            "Mach-O arm64"
        } else {
            "Mach-O x86_64"
        }
    } else if header.starts_with(&[0xca, 0xfe, 0xba, 0xbe]) {
        "Universal Mach-O"
    } else {
        ""
    };
    (kind.into(), arch.into())
}

pub(crate) fn bytes(path: &Path, offset: u64, length: usize) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = Vec::with_capacity(length.min(TEXT_LIMIT));
    file.take(length.min(TEXT_LIMIT) as u64)
        .read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub(crate) fn text(path: &Path) -> Result<String> {
    let bytes = bytes(path, 0, TEXT_LIMIT)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn hex(path: &Path, offset: u64) -> Result<String> {
    use std::fmt::Write;
    let data = bytes(path, offset, 4096)?;
    let mut output = String::new();
    for (row, chunk) in data.chunks(16).enumerate() {
        write!(output, "{:012x}  ", offset + (row * 16) as u64)?;
        for byte in chunk {
            write!(output, "{byte:02x} ")?;
        }
        for _ in chunk.len()..16 {
            output.push_str("   ");
        }
        output.push_str("  ");
        output.extend(chunk.iter().map(|byte| {
            if (32..127).contains(byte) {
                char::from(*byte)
            } else {
                '.'
            }
        }));
        output.push('\n');
    }
    Ok(output)
}
