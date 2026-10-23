use super::catalog::{Entry, Launch};
use crate::error::{Result, failure};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

pub(super) fn supported(entry: &Entry) -> bool {
    let dist = &entry.distribution;
    dist["npx"]["package"].is_string()
        || dist["uvx"]["package"].is_string()
        || dist["command"].is_string()
        || dist["binary"][super::catalog::platform()]["cmd"].is_string()
}

pub(crate) fn resolve(entry: &Entry, cancelled: &AtomicBool) -> Result<Launch> {
    if let Some(local) = native(entry) {
        return Ok(local);
    }
    let dist = &entry.distribution;
    for runner in ["npx", "uvx"] {
        if let Some(package) = dist[runner]["package"].as_str() {
            let mut args = Vec::new();
            if runner == "npx" {
                args.push("--yes".into());
            }
            args.push(unpinned(package));
            args.extend(strings(&dist[runner]["args"]));
            return Ok(Launch {
                command: crate::executable_paths::find(runner),
                args,
                env: environment(&dist[runner]["env"]),
            });
        }
    }
    if let Some(command) = dist["command"].as_str() {
        return Ok(Launch {
            command: crate::executable_paths::find(command),
            args: strings(&dist["args"]),
            env: BTreeMap::new(),
        });
    }
    let build = &dist["binary"][super::catalog::platform()];
    Ok(Launch {
        command: super::install::binary(entry, build, cancelled)?,
        args: strings(&build["args"]),
        env: environment(&build["env"]),
    })
}

pub(crate) fn installed(entry: &Entry) -> bool {
    if native(entry).is_some() {
        return true;
    }
    let own = match entry.id.as_str() {
        "claude-acp" => "claude",
        "codex-acp" => "codex",
        "gemini" => "gemini",
        "opencode" => "opencode",
        "goose" => "goose",
        "aster" => "aster",
        _ => "",
    };
    if !own.is_empty() && std::path::Path::new(&crate::executable_paths::find(own)).is_file() {
        return true;
    }
    let build = &entry.distribution["binary"][super::catalog::platform()];
    super::install::location(entry, build).is_ok_and(|path| path.is_file())
}

fn native(entry: &Entry) -> Option<Launch> {
    let (command, args): (&str, &[&str]) = match entry.id.as_str() {
        "claude-acp" => ("claude-agent-acp", &[]),
        "codex-acp" => ("codex-acp", &[]),
        "gemini" => ("gemini", &["--experimental-acp"]),
        "opencode" => ("opencode", &["acp"]),
        "aster" => ("aster", &["acp"]),
        _ => return None,
    };
    let command = crate::executable_paths::find(command);
    std::path::Path::new(&command).is_file().then(|| Launch {
        command,
        args: args.iter().map(|arg| (*arg).into()).collect(),
        env: BTreeMap::new(),
    })
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn environment(value: &Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.into())))
        .collect()
}

fn unpinned(package: &str) -> String {
    match package.rsplit_once('@') {
        Some((name, _)) if !name.is_empty() => name.into(),
        _ => package.into(),
    }
}

pub(crate) fn selected(id: &str, cancelled: &AtomicBool) -> Result<Entry> {
    super::catalog::load(cancelled)?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| failure("The selected agent is unavailable for this platform"))
}
