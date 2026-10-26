use crate::settings::permissions::{Level, Permissions, Tool};
use serde_json::{Value, json};

pub(super) fn response(params: &Value, permissions: &Permissions) -> Option<Value> {
    match level(params, permissions) {
        Level::Ask => None,
        Level::Allow => select(params, true),
        Level::Never => Some(select(params, false).unwrap_or_else(cancelled)),
    }
}

pub(super) fn level(params: &Value, permissions: &Permissions) -> Level {
    let title = params
        .pointer("/toolCall/title")
        .and_then(Value::as_str)
        .unwrap_or_default();
    permissions.level(category(title))
}

pub(super) fn select(params: &Value, allow: bool) -> Option<Value> {
    let kinds = if allow {
        ["allow_once", "allow_always"]
    } else {
        ["reject_once", "reject_always"]
    };
    for kind in kinds {
        if let Some(id) = params["options"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|option| option["kind"].as_str() == Some(kind))
            .and_then(|option| option["optionId"].as_str())
        {
            return Some(json!({"outcome":{"outcome":"selected","optionId":id}}));
        }
    }
    None
}

pub(super) fn cancelled() -> Value {
    json!({"outcome":{"outcome":"cancelled"}})
}

fn category(title: &str) -> Tool {
    if let Some(tool) = server_tool(title, "prism") {
        return match tool {
            "get_state"
            | "list_functions"
            | "read_bytes"
            | "analysis_tools"
            | "decompile_batch"
            | "semantic_search"
            | "index_function_search" => Tool::Read,
            "open_function"
            | "current_view"
            | "set_view"
            | "list_marks"
            | "show_screen"
            | prism::mcp::question::TOOL => Tool::View,
            "rename_function" => Tool::Rename,
            "add_note" | "add_comment" | "bookmark" | "set_code_type" | "set_number_format" => {
                Tool::Notes
            }
            "propose_patch" => Tool::Patch,
            "gpu_profile" => Tool::Gpu,
            "list_managed_types"
            | "decompile_managed_type"
            | "reconstruction_status"
            | "reconstruction_units"
            | "reconstruction_modules"
            | "reconstruction_files"
            | "reconstruction_evidence"
            | "reconstruction_functions"
            | "reconstruction_read_file"
            | "reconstruction_coverage"
            | "reconstruction_source_files" => Tool::Read,
            _ => Tool::Shell,
        };
    }
    if let Some(tool) = server_tool(title, "prism-analysis") {
        return match tool {
            "list_functions"
            | "read_bytes"
            | "analysis_tools"
            | "decompile_batch"
            | "semantic_search"
            | "index_function_search" => Tool::Read,
            _ => Tool::Shell,
        };
    }
    let tool = server_tool(title, "prism-rea").or_else(|| server_tool(title, "rea"));
    if tool.is_some_and(|tool| READ_ONLY_REA.iter().any(|prefix| tool.starts_with(prefix))) {
        Tool::Read
    } else {
        Tool::Shell
    }
}

fn server_tool<'a>(title: &'a str, server: &str) -> Option<&'a str> {
    if let Some(tool) = title.strip_prefix(&format!("mcp__{server}__")) {
        return Some(tool);
    }
    let suffix = title.strip_prefix(server)?;
    suffix
        .starts_with(['_', ':', '.'])
        .then(|| suffix.trim_start_matches(['_', ':', '.']))
}

const READ_ONLY_REA: &[&str] = &[
    "open_binary",
    "close_binary",
    "binary_overview",
    "binary_session",
    "list_",
    "procedure_",
    "read_",
    "search_",
    "inspect_",
    "xrefs",
    "get_",
    "find_",
    "analyze_",
    "demangle_",
    "address_",
    "current_",
    "resolve_",
    "compare_",
    "build_call_path",
];
