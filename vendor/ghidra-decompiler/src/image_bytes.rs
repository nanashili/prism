use crate::error::{Error, Result};
use std::io::{Read, Write};

#[cfg(unix)]
use memmap2::{Mmap, MmapOptions};

#[cfg(unix)]
use std::fs::File;

#[derive(Debug)]
enum Storage {
    Owned(Vec<u8>),
    #[cfg(unix)]
    Mapped {
        bytes: Mmap,
        _file: File,
    },
}

#[derive(Debug)]
pub struct ImageBytes {
    storage: Storage,
}

impl ImageBytes {
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self {
            storage: Storage::Owned(bytes),
        }
    }

    pub fn from_reader<R: Read>(reader: &mut R, length: u64) -> Result<Self> {
        let length =
            usize::try_from(length).map_err(|_| Error::Lowlevel("image is too large for this platform".into()))?;

        #[cfg(unix)]
        {
            let file = tempfile::tempfile()
                .map_err(|error| Error::Lowlevel(format!("cannot create image backing file: {error}")))?;
            file.set_len(length as u64)
                .map_err(|error| Error::Lowlevel(format!("cannot size image backing file: {error}")))?;
            let mut limited = reader.take(length as u64);
            let mut writer = &file;
            copy_exact(&mut limited, &mut writer, length)?;
            // SAFETY: The anonymous file is private and fully initialized. Its handle
            // stays owned alongside the mapping and is never exposed or mutated.
            let bytes = unsafe { MmapOptions::new().len(length).map(&file) }
                .map_err(|error| Error::Lowlevel(format!("cannot map image backing file: {error}")))?;
            Ok(Self {
                storage: Storage::Mapped { bytes, _file: file },
            })
        }

        #[cfg(not(unix))]
        {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(length)
                .map_err(|_| Error::Lowlevel("image allocation failed".into()))?;
            let mut limited = reader.take(length as u64);
            copy_exact(&mut limited, &mut bytes, length)?;
            Ok(Self::from_vec(bytes))
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match &self.storage {
            Storage::Owned(bytes) => bytes,
            #[cfg(unix)]
            Storage::Mapped { bytes, .. } => bytes,
        }
    }
}

impl AsRef<[u8]> for ImageBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

fn copy_exact<R: Read, W: Write>(reader: &mut R, writer: &mut W, length: usize) -> Result<()> {
    let mut buffer = [0u8; 64 * 1024];
    let mut copied = 0usize;
    while copied < length {
        let requested = (length - copied).min(buffer.len());
        let read = reader
            .read(&mut buffer[..requested])
            .map_err(|error| Error::Lowlevel(format!("cannot read image: {error}")))?;
        if read == 0 {
            return Err(Error::Lowlevel("image ended before the selected range".into()));
        }
        writer
            .write_all(&buffer[..read])
            .map_err(|error| Error::Lowlevel(format!("cannot write image backing file: {error}")))?;
        copied += read;
    }
    let mut extra = [0u8; 1];
    if reader
        .read(&mut extra)
        .map_err(|error| Error::Lowlevel(format!("cannot verify image length: {error}")))?
        != 0
    {
        return Err(Error::Lowlevel("image contains more bytes than declared".into()));
    }
    Ok(())
}
