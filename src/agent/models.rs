use super::options::{Choice, menus};
use crate::error::{Result, failure};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;

pub(crate) type Known = BTreeMap<String, Vec<Choice>>;

const MAX_BYTES: u64 = 1024 * 1024;
static ACCESS: Mutex<()> = Mutex::new(());

pub(crate) fn load() -> Result<Known> {
    read(&crate::settings::support_dir().join("agent-models.json"))
}

pub(super) fn remember(agent: &str, options: &serde_json::Value) -> Result<()> {
    let choices = choices(options);
    if choices.is_empty() {
        return Ok(());
    }
    let _access = ACCESS
        .lock()
        .map_err(|_| failure("Model storage is unavailable"))?;
    let mut saved = load()?;
    if saved.get(agent) == Some(&choices) {
        return Ok(());
    }
    saved.insert(agent.into(), choices);
    write(
        &crate::settings::support_dir().join("agent-models.json"),
        &saved,
    )
}

pub(crate) fn choices(options: &serde_json::Value) -> Vec<Choice> {
    menus(options)
        .into_iter()
        .find(|menu| menu.category == "model")
        .map(|menu| menu.choices)
        .unwrap_or_default()
}

fn read(path: &Path) -> Result<Known> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Known::new()),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Saved models exceed 1 MiB".into());
    }
    let saved = serde_json::from_slice(&bytes)?;
    validate(&saved)?;
    Ok(saved)
}

fn write(path: &Path, saved: &Known) -> Result<()> {
    validate(saved)?;
    let bytes = serde_json::to_vec_pretty(saved)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Saved models exceed 1 MiB".into());
    }
    crate::storage::atomic::write(path, |file| {
        use std::io::Write;
        file.write_all(&bytes)?;
        Ok(())
    })
}

fn validate(saved: &Known) -> Result<()> {
    if saved.len() > 128
        || saved.iter().any(|(agent, choices)| {
            agent.is_empty()
                || agent.len() > 4096
                || choices.len() > 256
                || choices.iter().any(|choice| {
                    choice.value.is_empty()
                        || choice.value.len() > 4096
                        || choice.name.len() > 1024
                        || choice.description.len() > 4096
                })
        })
    {
        return Err("Saved models exceed their supported limits".into());
    }
    Ok(())
}
