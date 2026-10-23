use crate::error::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub(super) fn fingerprint(path: &Path, include_digest: bool) -> Result<Value> {
    let path = path.canonicalize()?;
    let metadata = path.metadata()?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string();
    let digest = if include_digest {
        let mut file = File::open(&path)?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let length = file.read(&mut buffer)?;
            if length == 0 {
                break;
            }
            hash.update(&buffer[..length]);
        }
        Some(format!("{:x}", hash.finalize()))
    } else {
        None
    };
    Ok(json!({"path":path,"size":metadata.len(),"modified_ns":modified,"sha256":digest}))
}
