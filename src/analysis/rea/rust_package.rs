use crate::error::{Result, failure};
use std::path::{Path, PathBuf};

pub(super) fn validate(launcher: &Path) -> Result<()> {
    artifacts(launcher)?;
    let root = package_root(launcher)?;
    let package: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("package.json"))?)?;
    if package.get("name").and_then(serde_json::Value::as_str) != Some("rea-agents") {
        return Err(failure(
            "Choose the scripts/rea.mjs launcher from a built REA package",
        ));
    }
    if !root.join("node_modules").is_dir() {
        return Err(failure(format!(
            "The selected REA package needs its installed Node dependencies at {}. Run `npm ci` in that package before building it.",
            root.join("node_modules").display(),
        )));
    }
    Ok(())
}

pub(super) fn artifacts(launcher: &Path) -> Result<Vec<PathBuf>> {
    let root = package_root(launcher)?;
    let mut artifacts = vec![launcher.to_owned(), worker_protocol(launcher)?];

    for relative in [
        "package.json",
        "dist/generatedPackageMetadata.js",
        "dist/ghidra-rust/GhidraRustProvider.js",
        "dist/mcp/main.js",
        "dist/contracts/batchDecompileOutput.js",
        "dist/contracts/enhancedInputs.js",
        "dist/domain/analysisErrorProjection.js",
        "dist/application/EnhancedTools.js",
        "dist/application/FunctionAnalysisEvidence.js",
        "dist/application/CapabilityInventory.js",
        "dist/process/OwnedProcessResidentMemory.js",
        "dist/application/binary/BinarySession.js",
        "dist/application/binary/BinarySessionExecution.js",
        "dist/composition/binary.js",
        "dist/server/registerEnhancedTools.js",
    ] {
        artifacts.push(required_file(root.join(relative))?);
    }

    artifacts.extend(javascript_artifacts(&root.join("dist/ghidra-rust"))?);
    artifacts.extend(javascript_artifacts(&root.join("dist/mcp"))?);
    artifacts.sort();
    artifacts.dedup();
    Ok(artifacts)
}

pub(super) fn worker_protocol(launcher: &Path) -> Result<PathBuf> {
    required_file(package_root(launcher)?.join("dist/ghidra-rust/GhidraRustProtocol.js"))
}

pub(super) fn package_root(launcher: &Path) -> Result<&Path> {
    let root = launcher
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| failure("The configured Rust REA launcher has no package directory"))?;
    if launcher.file_name().is_none_or(|name| name != "rea.mjs")
        || launcher
            .parent()
            .and_then(Path::file_name)
            .is_none_or(|name| name != "scripts")
    {
        return Err(failure(
            "Choose the scripts/rea.mjs launcher from a built REA package",
        ));
    }
    Ok(root)
}

fn required_file(path: PathBuf) -> Result<PathBuf> {
    let metadata = path.metadata().map_err(|error| {
        failure(format!(
            "The configured Rust REA build cannot read {}: {error}. Supply a complete packaged runtime or, in a source checkout, run `npm ci` and `npm run build:cached`, then select its scripts/rea.mjs launcher in Settings > Engines.",
            path.display(),
        ))
    })?;
    if !metadata.is_file() {
        return Err(failure(format!(
            "The configured Rust REA build requires a regular file at {}",
            path.display(),
        )));
    }
    Ok(path)
}

fn javascript_artifacts(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut artifacts = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            artifacts.extend(javascript_artifacts(&path)?);
        } else if metadata.is_file() && path.extension().is_some_and(|extension| extension == "js")
        {
            artifacts.push(path);
        }
    }
    artifacts.sort();
    Ok(artifacts)
}
