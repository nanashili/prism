use crate::error::Result;

#[cfg(target_os = "macos")]
pub(super) fn resident(pid: u32) -> Result<Option<u64>> {
    let mut info = std::mem::MaybeUninit::<libc::proc_taskinfo>::uninit();
    let expected = i32::try_from(std::mem::size_of::<libc::proc_taskinfo>())?;
    // SAFETY: The writable, aligned buffer has exactly the size passed to proc_pidinfo.
    let written = unsafe {
        libc::proc_pidinfo(
            i32::try_from(pid)?,
            libc::PROC_PIDTASKINFO,
            0,
            info.as_mut_ptr().cast(),
            expected,
        )
    };
    if written == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        return Err(error.into());
    }
    if written != expected {
        return Err("The process memory query returned an incomplete record".into());
    }
    // SAFETY: The successful size check confirms the complete C record was initialized.
    Ok(Some(unsafe { info.assume_init() }.pti_resident_size))
}

#[cfg(target_os = "linux")]
pub(super) fn resident(pid: u32) -> Result<Option<u64>> {
    let text = match std::fs::read_to_string(format!("/proc/{pid}/status")) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let line = text.lines().find_map(|line| line.strip_prefix("VmRSS:"));
    let Some(value) = line.and_then(|line| line.split_whitespace().next()) else {
        return Ok(None);
    };
    Ok(Some(
        value
            .parse::<u64>()?
            .checked_mul(1024)
            .ok_or("Invalid process memory size")?,
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn resident(_: u32) -> Result<Option<u64>> {
    Ok(None)
}
