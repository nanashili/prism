use super::tool;
use serde_json::{Value, json};

pub(super) fn definitions() -> Vec<Value> {
    let mut tools = vec![
        tool(
            "open_file",
            "Open an app, binary, source, shader, model or Prism project by absolute path.",
            &["path"],
            json!({"path":{"type":"string","minLength":1,"maxLength":4096}}),
        ),
        tool(
            "analyze",
            "Index functions in the opened file using its configured analysis backend.",
            &[],
            json!({}),
        ),
        tool(
            "save_project",
            "Save the opened project to an absolute .prism path.",
            &["path"],
            json!({"path":{"type":"string","minLength":1,"maxLength":4096}}),
        ),
        tool(
            "get_state",
            "Read the current file, selection, panes, visible code and agent progress.",
            &[],
            json!({}),
        ),
        tool(
            "current_view",
            "Read the currently selected file, function, address and view.",
            &[],
            json!({}),
        ),
        tool(
            "list_marks",
            "Read saved function names, notes and bookmarks for the open project.",
            &[],
            json!({}),
        ),
        tool(
            "ask_agent",
            "Submit a prompt to the configured agent. Returns immediately; use get_state for progress. Fails while an operation is active.",
            &["prompt"],
            json!({"prompt":{"type":"string","minLength":1,"maxLength":32000}}),
        ),
        tool(
            "capture_window",
            "Save a PNG of the Prism window to an absolute path.",
            &["path"],
            json!({"path":{"type":"string","minLength":1,"maxLength":4096}}),
        ),
        tool(
            "open_function",
            "Select a function by address, original name or local name. Use function or address, not both. Index functions before using a name.",
            &[],
            json!({
                "function":{"type":"string","minLength":1,"maxLength":1024},
                "address":{"type":"string","minLength":1,"maxLength":1024},
                "view":{"type":"string","description":"Optional initial view, such as Assembly or Graph. Defaults to Pseudocode."},
                "split":{"type":"boolean","description":"Open in the other split pane and focus it."}
            }),
        ),
        tool(
            "show_screen",
            "Show Home, Settings, Setup, or the open Workbench. Settings accepts an optional page title. Does not change settings.",
            &["screen"],
            json!({"screen":{"type":"string","enum":["Home","Settings","Setup","Workbench"]},"page":{"type":"string","maxLength":128}}),
        ),
        tool(
            "set_view",
            "Select a workbench view, such as pseudocode, assembly, graph, hex or notes.",
            &["view"],
            json!({"view":{"type":"string","minLength":1,"maxLength":64}}),
        ),
        tool(
            "rename_function",
            "Set a local function name using a C identifier. Accepts an address, original name or local name.",
            &["function", "new_name"],
            json!({"function":{"type":"string","minLength":1,"maxLength":1024},"new_name":{"type":"string","minLength":1,"maxLength":1024}}),
        ),
        tool(
            "add_note",
            "Save a function note. Empty text removes the note. Accepts an address, original name or local name.",
            &["function", "note"],
            json!({"function":{"type":"string","minLength":1,"maxLength":1024},"note":{"type":"string","maxLength":65536}}),
        ),
        tool(
            "bookmark",
            "Bookmark a function address in the project.",
            &["address"],
            json!({"address":{"type":"string","minLength":1,"maxLength":1024}}),
        ),
    ];
    if let Some(function) = tools
        .iter_mut()
        .find(|tool| tool["name"] == "open_function")
    {
        function["inputSchema"]["oneOf"] = json!([
            {"required":["function"]}, {"required":["address"]}
        ]);
    }
    tools
}
