use super::{Mode, support_dir};
use serde_json::{Value, json};
use std::path::Path;

#[must_use]
/// Builds an agent server configuration using the helper beside the desktop executable.
pub fn configuration(desktop: &Path, mode: Mode) -> Value {
    let helper = desktop.with_file_name(if cfg!(windows) {
        "prism-mcp.exe"
    } else {
        "prism-mcp"
    });
    let (command, mut arguments) = if helper.is_file() {
        (helper, Vec::new())
    } else {
        (desktop.to_path_buf(), vec!["mcp"])
    };
    if let Some(argument) = mode.argument() {
        arguments.push(argument);
    }
    let environment: Vec<Value> = std::env::var_os("PRISM_SUPPORT")
        .map(|_| json!({"name":"PRISM_SUPPORT","value":support_dir()}))
        .into_iter()
        .collect();
    json!({
        "name":mode.name(),"command":command,"args":arguments,
        "env":environment
    })
}
