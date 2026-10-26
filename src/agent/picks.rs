use super::options::Pick;
use crate::error::Result;
use crate::settings::Settings;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;

type Saved = BTreeMap<String, BTreeMap<String, String>>;

pub(crate) fn key(settings: &Settings) -> String {
    if !settings.agent_id.is_empty() {
        return settings.agent_id.clone();
    }
    command_key(&settings.agent, &settings.agent_args)
}

pub(crate) fn command_key(command: &str, arguments: &str) -> String {
    if command.ends_with("codex-acp") || arguments.contains("codex-acp") {
        return "codex-acp".into();
    }
    format!("{command} {arguments}")
}

pub(super) fn load(agent: &str) -> Result<Vec<Pick>> {
    Ok(read()?
        .remove(agent)
        .unwrap_or_default()
        .into_iter()
        .map(|(id, value)| Pick {
            modes_api: id == "mode",
            id,
            value,
        })
        .collect())
}

pub(crate) fn remember(agent: &str, id: &str, value: Option<&str>) -> Result<()> {
    let mut saved = read()?;
    let picks = saved.entry(agent.into()).or_default();
    if let Some(value) = value {
        picks.insert(id.into(), value.into());
    } else {
        picks.remove(id);
    }
    write(&saved)
}

pub(crate) fn restore_if_current(
    agent: &str,
    id: &str,
    rejected: &str,
    previous: Option<&str>,
) -> Result<()> {
    let mut saved = read()?;
    let Some(picks) = saved.get_mut(agent) else {
        return Ok(());
    };
    if picks.get(id).map(String::as_str) != Some(rejected) {
        return Ok(());
    }
    if let Some(previous) = previous {
        picks.insert(id.into(), previous.into());
    } else {
        picks.remove(id);
    }
    write(&saved)
}

fn write(saved: &Saved) -> Result<()> {
    write_file(
        &crate::settings::support_dir().join("agent-picks.json"),
        saved,
    )
}

fn write_file(path: &Path, saved: &Saved) -> Result<()> {
    validate(saved)?;
    let encoded = serde_json::to_vec_pretty(saved)?;
    if encoded.len() > 1024 * 1024 {
        return Err("Saved agent options exceed 1 MiB".into());
    }
    crate::storage::atomic::write(path, |file| {
        file.write_all(&encoded)?;
        Ok(())
    })
}

fn read() -> Result<Saved> {
    read_file(&crate::settings::support_dir().join("agent-picks.json"))
}

fn read_file(path: &Path) -> Result<Saved> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Saved::new()),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err("Saved agent options exceed 1 MiB".into());
    }
    let saved = serde_json::from_slice(&bytes)?;
    validate(&saved)?;
    Ok(saved)
}

fn validate(saved: &Saved) -> Result<()> {
    if saved.len() > 128
        || saved.iter().any(|(agent, picks)| {
            agent.len() > 4096
                || picks.len() > 64
                || picks
                    .iter()
                    .any(|(id, value)| id.len() > 240 || value.len() > 4096)
        })
    {
        return Err("Saved agent options exceed their supported limits".into());
    }
    Ok(())
}
