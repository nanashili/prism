use crate::settings::Settings;

pub(super) const GROUPS: &[&str] = &["Navigate", "Annotate", "View", "Agent", "Debugger"];
pub(super) const COMMANDS: &[(&str, &str, &str, &str)] = &[
    ("goto", "Go to address or symbol", "Navigate", "g"),
    ("palette", "Search everything", "Navigate", "cmd-k"),
    ("open", "Open a file", "Navigate", "cmd-o"),
    ("save", "Save project", "Navigate", "cmd-s"),
    ("back", "Back", "Navigate", "cmd-["),
    ("forward", "Forward", "Navigate", "cmd-]"),
    ("find", "Find in this view", "Navigate", "cmd-f"),
    ("findNext", "Find next", "Navigate", "cmd-g"),
    ("findPrevious", "Find previous", "Navigate", "shift-cmd-g"),
    ("confirm", "Confirm", "Navigate", "enter"),
    ("dismiss", "Dismiss", "Navigate", "escape"),
    ("rename", "Rename", "Annotate", "n"),
    ("comment", "Comment", "Annotate", ";"),
    ("xrefs", "Cross-references", "Annotate", "x"),
    ("bookmark", "Bookmark", "Annotate", "cmd-d"),
    ("undo", "Undo the last change", "Annotate", "cmd-z"),
    ("redo", "Redo", "Annotate", "shift-cmd-z"),
    ("patch", "Patch bytes here", "Annotate", "shift-cmd-b"),
    ("markCode", "Mark as code", "Annotate", "c"),
    ("markProcedure", "Mark as a function", "Annotate", "p"),
    ("markData", "Mark as data", "Annotate", "d"),
    ("markString", "Mark as a C string", "Annotate", "a"),
    ("markUndefined", "Mark as undefined", "Annotate", "u"),
    (
        "showHex",
        "Show number as hexadecimal",
        "Annotate",
        "shift-h",
    ),
    (
        "showDecimal",
        "Show number as decimal",
        "Annotate",
        "shift-q",
    ),
    (
        "showCharacter",
        "Show number as a character",
        "Annotate",
        "shift-r",
    ),
    (
        "showOffset",
        "Show number as an offset",
        "Annotate",
        "alt-z",
    ),
    (
        "showAddress",
        "Show number as an address",
        "Annotate",
        "shift-z",
    ),
    ("graph", "Graph view", "View", "space"),
    ("pseudo", "Pseudocode", "View", "alt-enter"),
    ("strings", "Strings", "View", "shift-cmd-s"),
    ("split", "Split right", "View", "cmd-\\"),
    ("splitDown", "Split down", "View", "shift-cmd-\\"),
    ("terminal", "Terminal", "View", "ctrl-`"),
    ("details", "Show or hide details", "View", "cmd-j"),
    ("selectAll", "Select all code", "View", "cmd-a"),
    ("copy", "Copy selected code", "View", "cmd-c"),
    ("agent", "Agent panel", "Agent", "cmd-l"),
    ("thread", "New chat", "Agent", "cmd-n"),
    ("stop", "Stop the agent", "Agent", ""),
    ("settings", "Settings", "Agent", "cmd-,"),
    ("debug", "Debug this program", "Debugger", "cmd-r"),
    ("continue", "Continue", "Debugger", "ctrl-cmd-y"),
    ("stepOver", "Step over", "Debugger", "f6"),
    ("stepIn", "Step into", "Debugger", "f7"),
    ("stepOut", "Step out", "Debugger", "f8"),
];

pub(super) fn supported(id: &str) -> bool {
    matches!(
        id,
        "open"
            | "save"
            | "thread"
            | "palette"
            | "settings"
            | "undo"
            | "back"
            | "forward"
            | "rename"
            | "comment"
            | "xrefs"
            | "bookmark"
            | "graph"
            | "pseudo"
            | "strings"
            | "split"
            | "agent"
            | "stop"
            | "patch"
            | "debug"
    )
}

pub(super) fn combo<'a>(settings: &'a Settings, id: &str, default: &'a str) -> &'a str {
    if let Some(value) = settings.views.shortcuts.get(id) {
        return value;
    }
    match (settings.views.keymap.as_str(), id) {
        ("Hopper" | "Cursor or VS Code" | "Zed", "palette") => "shift-cmd-p",
        ("Hopper", "back") => "backspace",
        ("Hopper", "bookmark") => "cmd-f6",
        ("Ghidra", "palette") => "cmd-3",
        ("Ghidra", "back") => "alt-left",
        ("Ghidra", "forward") => "alt-right",
        ("Ghidra", "rename") => "l",
        ("Ghidra", "xrefs") => "shift-cmd-f",
        ("Ghidra", "graph") => "cmd-space",
        ("Ghidra", "pseudo") => "cmd-e",
        ("Cursor or VS Code" | "Zed" | "Xcode", "goto") => "shift-cmd-o",
        ("Cursor or VS Code" | "Zed", "back") => "ctrl--",
        ("Cursor or VS Code" | "Zed", "forward") => "ctrl-shift--",
        ("Cursor or VS Code" | "Zed", "rename") => "f2",
        ("Cursor or VS Code" | "Zed" | "Xcode", "comment") => "cmd-/",
        ("Cursor or VS Code", "xrefs") => "shift-f12",
        ("Zed", "xrefs") => "alt-shift-f12",
        ("Xcode", "palette") => "shift-cmd-a",
        ("Xcode", "agent") => "alt-cmd-a",
        ("Xcode", "back") => "ctrl-cmd-left",
        ("Xcode", "forward") => "ctrl-cmd-right",
        _ => default,
    }
}
