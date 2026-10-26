use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

const REGISTRY: &str = "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";
const FEATURED: [&str; 8] = [
    "claude-acp",
    "codex-acp",
    "gemini",
    "github-copilot-cli",
    "cursor",
    "goose",
    "opencode",
    "aster",
];

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Entry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub icon: String,
    pub distribution: Value,
}

#[derive(Default)]
pub(crate) struct Launch {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub(crate) fn load(cancelled: &AtomicBool) -> Result<Vec<Entry>> {
    let file = crate::settings::support_dir().join("agents/registry.json");
    let fresh = std::fs::metadata(&file)
        .and_then(|info| info.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age.as_secs() < 86400);
    if !fresh {
        let download = super::download::cached(REGISTRY, &file, 2 * 1024 * 1024, cancelled);
        if let Err(error) = download {
            eprintln!("Prism: using cached agent registry: {error}");
        }
    }
    let mut entries = if file.is_file() {
        #[derive(Deserialize)]
        struct Registry {
            agents: Vec<Entry>,
        }
        let reader = std::io::BufReader::new(std::fs::File::open(file)?);
        serde_json::from_reader::<_, Registry>(reader)?.agents
    } else {
        fallback()
    };
    entries.retain(|entry| valid_id(&entry.id) && super::launch::supported(entry));
    if !entries.iter().any(|entry| entry.id == "aster") {
        entries.push(local());
    }
    for entry in &mut entries {
        entry.name = short_name(&entry.name);
    }
    entries.sort_by(|a, b| {
        rank(&a.id)
            .cmp(&rank(&b.id))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

pub(crate) fn download_icon(
    url: &str,
    destination: &std::path::Path,
    cancelled: &AtomicBool,
) -> Result<()> {
    super::download::cached(url, destination, 128 * 1024, cancelled)
}

pub(crate) fn fallback() -> Vec<Entry> {
    [
        (
            "claude-acp",
            "Claude",
            json!({"npx":{"package":"@agentclientprotocol/claude-agent-acp"}}),
        ),
        (
            "codex-acp",
            "Codex",
            json!({"npx":{"package":"@agentclientprotocol/codex-acp"}}),
        ),
        (
            "gemini",
            "Gemini",
            json!({"npx":{"package":"@google/gemini-cli", "args":["--acp"]}}),
        ),
        (
            "github-copilot-cli",
            "GitHub Copilot",
            json!({"npx":{"package":"@github/copilot", "args":["--acp"]}}),
        ),
        (
            "cursor",
            "Cursor",
            json!({"command":"cursor-agent", "args":["acp"]}),
        ),
        ("goose", "Goose", json!({"command":"goose", "args":["acp"]})),
        (
            "opencode",
            "OpenCode",
            json!({"command":"opencode", "args":["acp"]}),
        ),
    ]
    .into_iter()
    .map(|(id, name, distribution)| Entry {
        id: id.into(),
        name: name.into(),
        description: String::new(),
        version: "local".into(),
        icon: String::new(),
        distribution,
    })
    .chain(std::iter::once(local()))
    .collect()
}

fn local() -> Entry {
    Entry {
        id: "aster".into(),
        name: "Aster".into(),
        description: "Your local Aster agent".into(),
        version: "local".into(),
        icon: String::new(),
        distribution: json!({"command":"aster", "args":["acp"]}),
    }
}

pub(crate) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() < 100
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub(crate) fn platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    format!("{os}-{}", std::env::consts::ARCH)
}

fn rank(id: &str) -> usize {
    FEATURED
        .iter()
        .position(|known| *known == id)
        .unwrap_or(FEATURED.len())
}

fn short_name(name: &str) -> String {
    for suffix in [" Agent", " ACP", " CLI"] {
        if name
            .to_ascii_lowercase()
            .ends_with(&suffix.to_ascii_lowercase())
        {
            return name[..name.len() - suffix.len()].trim().into();
        }
    }
    name.into()
}
