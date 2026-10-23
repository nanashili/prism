use super::{java_runtime, launch};
use crate::error::Result;
use crate::settings::AnalysisBackend;
use crate::settings::Settings;
use fingerprint::fingerprint;
use serde_json::{Value, json};
use std::path::Path;

mod fingerprint;

pub(crate) fn backend_identity(settings: &Settings) -> Result<Value> {
    let backend = settings.analysis_backend();
    backend.require_available()?;
    let rea = launch::rea_executable(settings)?;
    let node = launch::script_node(settings, &rea)?;
    if backend == AnalysisBackend::RustGhidraRea {
        return rust_identity(settings, &rea, node.as_deref());
    }
    let runtime = java_runtime::runtime_paths(settings)?;
    let mut components = rea_components(&rea)?;
    components.extend(runtime_components(&runtime)?);
    let node = node
        .as_deref()
        .map(|path| fingerprint(path, false))
        .transpose()?;

    Ok(json!({
        "contract":"prism-rea-ghidra-v1",
        "provider":"ghidra",
        "components":components,
        "node":node,
        "ghidra_directory":runtime.ghidra,
        "java_home":runtime.java,
        "memory_mib":settings.analysis_memory_mib,
        "workers":settings.tools.ghidra_threads,
        "startup_seconds":java_runtime::startup_timeout(settings)?.as_secs(),
        "request_seconds":settings.timeout_seconds,
        "disabled_analyzers":java_runtime::disabled_analyzers(settings),
        "language_id":std::env::var("REA_GHIDRA_LANGUAGE_ID").ok(),
        "compiler_spec_id":std::env::var("REA_GHIDRA_COMPILER_SPEC_ID").ok(),
        "open_policy":"explicit-preparation-session-only"
    }))
}

fn rea_components(rea: &Path) -> Result<Vec<Value>> {
    let root = rea.parent().and_then(Path::parent);
    let mut components = vec![fingerprint(rea, true)?];
    if let Some(root) = root {
        for relative in [
            "package.json",
            "dist/application/native/BatchDecompilation.js",
            "dist/application/binary/BinarySession.js",
            "dist/application/binary/BinarySessionExecution.js",
            "dist/ghidra/GhidraAnalysisCache.js",
            "dist/ghidra/GhidraHeadlessArguments.js",
            "dist/ghidra/GhidraClient.js",
            "dist/ghidra/GhidraProviderClient.js",
            "dist/ghidra/GhidraSessionError.js",
        ] {
            let path = root.join(relative);
            if path.is_file() {
                components.push(fingerprint(&path, true)?);
            }
        }
        let bridge = root.join("bridge/ghidra");
        if bridge.is_dir() {
            let mut paths = std::fs::read_dir(bridge)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<std::io::Result<Vec<_>>>()?;
            paths.sort();
            for path in paths
                .into_iter()
                .filter(|path| path.extension().is_some_and(|value| value == "java"))
            {
                components.push(fingerprint(&path, true)?);
            }
        }
    }
    Ok(components)
}

fn runtime_components(runtime: &java_runtime::RuntimePaths) -> Result<Vec<Value>> {
    let mut components = vec![fingerprint(
        &runtime.ghidra.join("Ghidra/application.properties"),
        true,
    )?];
    for relative in [
        "release",
        if cfg!(windows) {
            "bin/java.exe"
        } else {
            "bin/java"
        },
    ] {
        let path = runtime.java.join(relative);
        if path.is_file() {
            components.push(fingerprint(&path, relative == "release")?);
        }
    }
    Ok(components)
}

fn rust_identity(settings: &Settings, rea: &Path, node: Option<&Path>) -> Result<Value> {
    let worker = super::rust_runtime::worker_executable(settings, rea)?;
    let protocol = super::rust_package::worker_protocol(rea)?;
    let components = super::rust_package::artifacts(rea)?
        .iter()
        .map(|path| fingerprint(path, true))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "contract":"prism-rea-ghidra-rust-external-v2",
        "provider":"ghidra-rust",
        "components":components,
        "node":node.map(|path| fingerprint(path, false)).transpose()?,
        "worker":fingerprint(&worker, true)?,
        "worker_protocol":fingerprint(&protocol, true)?,
        "worker_argument":"--rea-rust-ghidra-worker",
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
