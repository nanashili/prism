use crate::error::{Result, failure};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::sync::Mutex;

type Answers = BTreeMap<String, BTreeSet<String>>;

// Approval reads and writes share one lock so concurrent chats cannot lose an answer.
static ACCESS: Mutex<()> = Mutex::new(());
const MAX_BYTES: usize = 1024 * 1024;

pub(super) fn allows(project: &str, tool: &str) -> Result<bool> {
    let _access = ACCESS
        .lock()
        .map_err(|_| failure("Approval storage is unavailable"))?;
    Ok(read()?
        .get(project)
        .is_some_and(|tools| tools.contains(tool)))
}

pub(super) fn remember(project: &str, tool: &str) -> Result<()> {
    let _access = ACCESS
        .lock()
        .map_err(|_| failure("Approval storage is unavailable"))?;
    let mut answers = read()?;
    let tools = answers.entry(project.into()).or_default();
    if !tools.insert(tool.into()) {
        return Ok(());
    }
    write(&answers)
}

pub(crate) fn forget() -> Result<()> {
    let _access = ACCESS
        .lock()
        .map_err(|_| failure("Approval storage is unavailable"))?;
    write(&Answers::new())
}

fn read() -> Result<Answers> {
    let path = crate::settings::support_dir().join("remembered-answers.json");
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Answers::new()),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err("Remembered approval answers exceed 1 MiB".into());
    }
    let answers = serde_json::from_slice(&bytes)?;
    validate(&answers)?;
    Ok(answers)
}

fn write(answers: &Answers) -> Result<()> {
    validate(answers)?;
    let encoded = serde_json::to_vec(answers)?;
    if encoded.len() > MAX_BYTES {
        return Err("Remembered approval answers exceed 1 MiB".into());
    }
    let path = crate::settings::support_dir().join("remembered-answers.json");
    crate::storage::atomic::write(&path, |file| {
        file.write_all(&encoded)?;
        Ok(())
    })
}

fn validate(answers: &Answers) -> Result<()> {
    if answers.len() > 256
        || answers.iter().any(|(project, tools)| {
            project.is_empty()
                || project.len() > 4096
                || tools.len() > 128
                || tools
                    .iter()
                    .any(|tool| tool.is_empty() || tool.len() > 4096)
        })
    {
        return Err("Remembered approval answers exceed their supported limits".into());
    }
    Ok(())
}
