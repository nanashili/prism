use super::Mode;
use serde_json::{Value, json};
use std::sync::OnceLock;

mod analysis;
mod analysis_repair;
mod decompilation;
mod execution;
mod investigation;
mod reconstruction;
mod reconstruction_modules;
mod type_recovery;
mod verification;
mod window;

static COMBINED: OnceLock<Value> = OnceLock::new();
static WINDOW: OnceLock<Value> = OnceLock::new();
static ANALYSIS: OnceLock<Value> = OnceLock::new();
static HEADLESS: OnceLock<Value> = OnceLock::new();

#[must_use]
/// Builds the MCP tool catalog for the selected application mode.
pub fn definitions(mode: Mode) -> Value {
    catalog(mode).clone()
}

pub(super) fn find_definition(mode: Mode, name: &str) -> Option<&'static Value> {
    catalog(mode)["tools"]
        .as_array()?
        .iter()
        .find(|tool| tool["name"].as_str() == Some(name))
}

fn catalog(mode: Mode) -> &'static Value {
    let cache = match mode {
        Mode::Combined => &COMBINED,
        Mode::Window => &WINDOW,
        Mode::Analysis => &ANALYSIS,
        Mode::Headless => &HEADLESS,
    };
    cache.get_or_init(|| build_definitions(mode))
}

fn build_definitions(mode: Mode) -> Value {
    let mut tools = Vec::new();
    tools.extend(verification::definitions());
    tools.push(investigation::definition());
    if matches!(mode, Mode::Combined | Mode::Window) {
        tools.extend(window::definitions());
        tools.push(super::question::definition());
    }
    if mode != Mode::Window {
        tools.extend(analysis::definitions());
    }
    if mode != Mode::Analysis {
        tools.extend(reconstruction::definitions());
        tools.extend(reconstruction_modules::definitions());
    }
    json!({"tools":tools})
}

#[must_use]
/// Identifies tools dispatched to the desktop event loop.
pub fn window_tool(name: &str) -> bool {
    matches!(
        name,
        "open_file"
            | "analyze"
            | "save_project"
            | "get_state"
            | "current_view"
            | "list_marks"
            | "ask_agent"
            | "capture_window"
            | "open_function"
            | "set_view"
            | "show_screen"
            | "transfer_version_annotations"
            | "rename_function"
            | "add_note"
            | "bookmark"
            | super::question::TOOL
    )
}

pub(super) fn tool(name: &str, description: &str, required: &[&str], properties: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":{
        "type":"object","properties":properties,"required":required,
        "additionalProperties":false
    }})
}
