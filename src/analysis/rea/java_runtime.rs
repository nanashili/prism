use crate::error::{Result, failure};
use crate::settings::{Settings, support_dir};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub(super) struct RuntimePaths {
    pub ghidra: PathBuf,
    pub java: PathBuf,
}

pub(super) fn runtime_paths(settings: &Settings) -> Result<RuntimePaths> {
    Ok(RuntimePaths {
        ghidra: required_directory(
            &settings.tools.ghidra_dir,
            "Ghidra installation",
            Path::new("Ghidra/application.properties"),
        )?,
        java: required_directory(
            &settings.tools.java_home,
            "Java home",
            Path::new(if cfg!(windows) {
                "bin/java.exe"
            } else {
                "bin/java"
            }),
        )?,
    })
}

fn required_directory(configured: &str, label: &str, marker: &Path) -> Result<PathBuf> {
    let directory = Path::new(configured);
    if configured.is_empty() || !directory.join(marker).is_file() {
        return Err(failure(format!(
            "Ghidra (Java through REA) requires an explicit {label} in Settings > Engines. Choose the directory containing {}.",
            marker.display(),
        )));
    }
    Ok(directory.canonicalize()?)
}

pub(super) fn configure_ghidra(
    command: &mut Command,
    settings: &Settings,
    prepared_java: bool,
) -> Result<()> {
    let workers = if prepared_java {
        settings.tools.ghidra_threads
    } else {
        1
    };
    command.env("REA_GHIDRA_DECOMPILER_THREADS", workers.to_string());
    for name in ["GHIDRA_MAXMEM", "GHIDRA_HEADLESS_MAXMEM"] {
        command.env(name, format!("{}M", settings.analysis_memory_mib));
    }
    command.env("REA_GHIDRA_CACHE_DIR", support_dir().join("ghidra"));
    if !prepared_java && !settings.tools.ghidra_dir.is_empty() {
        command.env("GHIDRA_INSTALL_DIR", &settings.tools.ghidra_dir);
    }
    if prepared_java {
        let runtime = runtime_paths(settings)?;
        command.env("GHIDRA_INSTALL_DIR", runtime.ghidra);
        command.env("JAVA_HOME", runtime.java);
        command.env(
            "REA_GHIDRA_STARTUP_TIMEOUT_MS",
            startup_timeout(settings)?.as_millis().to_string(),
        );
        command.env(
            "REA_GHIDRA_DISABLED_ANALYZERS",
            disabled_analyzers(settings).join(","),
        );
        command.env("REA_MCP_MAX_RESPONSE_BYTES", (10 * 1024 * 1024).to_string());
    }
    Ok(())
}

pub(super) fn startup_timeout(settings: &Settings) -> Result<Duration> {
    let minutes = match settings.tools.ghidra_analysis_time.as_str() {
        "30 minutes" => 30,
        "1 hour" => 60,
        "2 hours" => 120,
        "4 hours" => 240,
        "6 hours" => 360,
        "8 hours" => 480,
        "12 hours" => 720,
        _ => {
            return Err(failure(
                "Choose a supported Ghidra preparation time in Settings > Engines",
            ));
        }
    };
    Ok(Duration::from_secs(minutes * 60))
}

pub(super) fn disabled_analyzers(settings: &Settings) -> Vec<&'static str> {
    let tools = &settings.tools;
    let mut disabled = Vec::new();
    for (enabled, name) in [
        (tools.find_parameters, "Decompiler Parameter ID"),
        (tools.rebuild_switches, "Decompiler Switch Analysis"),
        (tools.hidden_code, "Aggressive Instruction Finder"),
    ] {
        if !enabled {
            disabled.push(name);
        }
    }
    if tools.fast_ghidra_analysis {
        disabled.push("Basic Constant Reference Analyzer");
    }
    if !tools.swift_names {
        disabled.push("Demangler Swift");
    }
    disabled
}
