use crate::error::{Result, failure};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub(super) fn validate(launcher: &Path) -> Result<()> {
    validate_executable_file(launcher)?;

    let canonical_launcher = launcher.canonicalize()?;
    reject_prism_executable(&canonical_launcher)?;
    validate_native_header(&canonical_launcher, launcher)
}

fn validate_executable_file(launcher: &Path) -> Result<()> {
    let metadata = launcher.metadata().map_err(|error| {
        failure(format!(
            "Cannot read the native REA executable at {}: {error}",
            launcher.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(failure(
            "The configured native REA path must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(failure(format!(
                "The native REA executable at {} is not executable",
                launcher.display()
            )));
        }
    }

    Ok(())
}

fn reject_prism_executable(launcher: &Path) -> Result<()> {
    let current_executable = std::env::current_exe()?.canonicalize()?;
    if launcher == current_executable {
        return Err(failure("The native REA executable cannot be Prism itself"));
    }
    Ok(())
}

fn validate_native_header(launcher: &Path, display_path: &Path) -> Result<()> {
    if !has_native_header(launcher)? {
        return Err(failure(format!(
            "The selected REA path at {} is not a native Mach-O, ELF, or PE binary. Download the matching nanashili/rea-rust release and select its `rea` executable.",
            display_path.display()
        )));
    }

    Ok(())
}

pub(super) fn has_native_header(launcher: &Path) -> Result<bool> {
    let mut magic = [0; 4];
    let mut file = File::open(launcher)?;
    match file.read_exact(&mut magic) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(false),
        Err(error) => {
            return Err(failure(format!(
                "Cannot read the native REA executable header at {}: {error}",
                launcher.display()
            )));
        }
    }

    let is_native = magic == *b"\x7fELF"
        || magic[..2] == *b"MZ"
        || matches!(
            magic,
            [0xfe, 0xed, 0xfa, 0xce | 0xcf]
                | [0xce | 0xcf, 0xfa, 0xed, 0xfe]
                | [0xca, 0xfe, 0xba, 0xbe | 0xbf]
                | [0xbe | 0xbf, 0xba, 0xfe, 0xca]
        );
    Ok(is_native)
}
