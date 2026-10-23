use crate::error::{Result, failure};
use serde_json::{Value, json};

pub(super) fn describe(language: &str) -> Result<Value> {
    let key = language.trim().to_ascii_lowercase();
    let (canonical, extension, validation_kind, artifact) = match key.as_str() {
        "rust" => ("Rust", "rs", "checked", false),
        "c" => ("C", "c", "built", true),
        "c++" => ("C++", "cpp", "built", true),
        "c#" => ("C#", "cs", "built", true),
        "swift" => ("Swift", "swift", "built", true),
        "go" => ("Go", "go", "built", true),
        "typescript" => ("TypeScript", "ts", "checked", false),
        "java" => ("Java", "java", "built", true),
        "zig" => ("Zig", "zig", "built", true),
        "python" => ("Python", "py", "checked", false),
        _ if !key.is_empty() => (language.trim(), "", "custom", false),
        _ => return Err(failure("Choose a target language")),
    };
    Ok(json!({
        "language": canonical,
        "source_extension": extension,
        "stages": ["evidence", "contract", "emit", "validate"],
        "build_check_available": validation_kind != "custom",
        "default_validation": validation_kind,
        "build_artifact_available": artifact,
        "behavior_verified": false,
    }))
}

pub(super) fn validation_kind(language: &str, custom: bool) -> &'static str {
    if custom {
        return "custom";
    }
    match language.trim().to_ascii_lowercase().as_str() {
        "rust" | "typescript" | "python" => "checked",
        "c" | "c++" | "c#" | "swift" | "go" | "java" | "zig" => "built",
        _ => "custom",
    }
}
