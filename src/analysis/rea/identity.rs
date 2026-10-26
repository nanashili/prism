use super::launch;
use crate::error::Result;
use crate::settings::Settings;
use fingerprint::fingerprint;
use serde_json::{Value, json};

mod fingerprint;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn backend_identity(settings: &Settings) -> Result<Value> {
    let backend = settings.analysis_backend();
    backend.require_available()?;
    let rea = launch::rea_executable(settings)?;
    let worker = super::rust_runtime::worker_executable(settings, &rea)?;
    let rea_identity = fingerprint(&rea, FingerprintDetail::Digest)?;
    let worker_identity = fingerprint(&worker, FingerprintDetail::Digest)?;
    let components = [&rea_identity, &worker_identity];
    Ok(json!({
        "contract":"prism-rea-ghidra-rust-native-v3",
        "provider":"ghidra-rust",
        "components":components,
        "worker":worker_identity,
        "runtime":"native",
        "workers":settings.tools.ghidra_threads,
        "cache_directory":super::rust_runtime::cache_dir()?,
        "memory_enforcement":"sampled_rss_termination_threshold",
        "memory_supervision":"sampled_worker_rss",
        "memory_limit_mib":settings.analysis_memory_mib,
        "memory_limit_scope":"worker_process_only",
        "memory_sample_interval_ms":1000,
        "memory_sample_timeout_ms":1000,
        "memory_hard_limit":false,
        "request_seconds":settings.timeout_seconds,
        "open_policy":"explicit-preparation-session-only"
    }))
}

use fingerprint::FingerprintDetail;
