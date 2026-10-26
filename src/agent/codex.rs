use super::{Prompt, catalog::Launch, picks};
use crate::error::Result;
use serde_json::{Value, json};

pub(super) fn selected(prompt: &Prompt) -> bool {
    picks::key(&prompt.settings) == "codex-acp"
}

pub(super) fn capabilities(prompt: &Prompt) -> Value {
    let mut value = json!({"session":{"notices":{}},"subagents":{}});
    if selected(prompt) {
        value["_meta"] = json!({"jetbrains":{"air":{
            "version":1,"capabilities":["nativeSubagentSessions"]
        }}});
    }
    value
}

pub(super) fn session(prompt: &Prompt) -> Value {
    if !selected(prompt) {
        return json!({"cwd":prompt.cwd,"mcpServers":prompt.servers});
    }
    let instructions = custom_instructions(prompt);
    json!({"cwd":prompt.cwd,"mcpServers":[],"_meta":{"jetbrains":{"air":{
        "version":1,"customInstructions":instructions
    }}}})
}

fn custom_instructions(prompt: &Prompt) -> String {
    format!(
        "{}\n{}\n\nPrevious conversation:\n{}",
        prompt.context.static_instructions, prompt.context.dynamic_context, prompt.history,
    )
}

/// Preserve Codex's configured servers while supplying Prism's native timeouts.
/// ACP's generic MCP conversion discards those timeout settings.
pub(super) fn configure_launch(launch: &mut Launch, prompt: &Prompt) -> Result<()> {
    if !selected(prompt) || prompt.servers.is_empty() {
        return Ok(());
    }
    let inherited = std::env::var("CODEX_CONFIG").ok();
    let encoded = launch.env.get("CODEX_CONFIG").or(inherited.as_ref());
    let mut config = match encoded {
        Some(encoded) if encoded.len() <= 1024 * 1024 => serde_json::from_str::<Value>(encoded)?,
        Some(_) => return Err("CODEX_CONFIG exceeds 1 MiB".into()),
        None => json!({}),
    };
    let root = config
        .as_object_mut()
        .ok_or("CODEX_CONFIG must contain a JSON object")?;
    let servers = root
        .entry("mcp_servers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("CODEX_CONFIG.mcp_servers must contain a JSON object")?;
    for server in &prompt.servers {
        let name = server["name"]
            .as_str()
            .ok_or("The MCP server has no name")?;
        servers.insert(
            name.into(),
            server_config(server, prompt.settings.timeout_seconds)?,
        );
    }
    launch
        .env
        .insert("CODEX_CONFIG".into(), serde_json::to_string(&config)?);
    Ok(())
}

fn server_config(server: &Value, timeout: u64) -> Result<Value> {
    let command = server["command"]
        .as_str()
        .ok_or("The MCP server has no command")?;
    let mut env = serde_json::Map::new();
    for variable in server["env"].as_array().into_iter().flatten() {
        let name = variable["name"]
            .as_str()
            .ok_or("An MCP environment variable has no name")?;
        let value = variable["value"]
            .as_str()
            .ok_or("An MCP environment variable has no value")?;
        env.insert(name.into(), value.into());
    }
    let mut config = json!({"command":command,"args":server["args"],"env":env});
    if matches!(server["name"].as_str(), Some("prism" | "prism-analysis")) {
        config["startup_timeout_sec"] = 120.into();
        config["tool_timeout_sec"] = timeout.saturating_add(60).into();
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::session;
    use crate::agent::{Prompt, PromptContext};
    use crate::settings::Settings;
    use std::path::PathBuf;

    #[test]
    fn session_new_delivers_static_and_dynamic_context_once() {
        let settings = Settings {
            agent: "codex-acp".into(),
            ..Settings::default()
        };
        let prompt = Prompt {
            chat: 1,
            project: None,
            file: None,
            text: "question".into(),
            attachments: Vec::new(),
            context: PromptContext {
                static_instructions: "static".into(),
                dynamic_context: "dynamic".into(),
            },
            history: "history".into(),
            cwd: PathBuf::new(),
            settings,
            servers: Vec::new(),
            picks: Vec::new(),
        };

        let params = session(&prompt);
        let instructions = params["_meta"]["jetbrains"]["air"]["customInstructions"]
            .as_str()
            .expect("Codex session parameters include custom instructions");

        assert_eq!(instructions.matches("static").count(), 1);
        assert_eq!(instructions.matches("dynamic").count(), 1);
        assert_eq!(instructions.matches("history").count(), 1);
    }
}
