use super::Mode;
use serde_json::{Value, json};

mod analysis;
mod decompilation;
mod execution;
mod reconstruction;
mod reconstruction_modules;
mod window;

pub fn definitions(mode: Mode) -> Value {
    let mut tools = Vec::new();
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
