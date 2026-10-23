use std::collections::BTreeSet;

pub(super) fn registration(name: &str) -> Option<&str> {
    name.trim_start_matches('_')
        .strip_prefix("RegisterModule_")
        .filter(|name| valid_module(name))
}

fn valid_module(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

pub(super) fn marker<'a>(name: &'a str, modules: &BTreeSet<String>) -> Option<Option<&'a str>> {
    let name = name.trim_start_matches('_');
    for prefix in [
        "RegisterModule_",
        "RegisterModuleICalls_",
        "RegisterModuleClassesGenerated_",
        "InitializeModule_",
        "CleanupModule_",
        "InternalInitializeModule_",
        "InternalCleanupModule_",
    ] {
        if let Some(module) = name
            .strip_prefix(prefix)
            .and_then(|name| name.split("::").next())
        {
            return modules.contains(module).then_some(Some(module));
        }
    }
    for prefix in ["Initialize", "Cleanup", "Get"] {
        if let Some(module) = name.strip_prefix(prefix).and_then(|name| {
            name.strip_suffix("ScriptingClasses")
                .or_else(|| name.strip_suffix("ModuleSpec"))
        }) {
            return modules.contains(module).then_some(Some(module));
        }
    }
    let unit = name.strip_prefix("GLOBAL__sub_I_")?;
    if let Some(unit) = unit.strip_prefix("Modules_") {
        let module = unit.split('_').next()?;
        return Some(modules.contains(module).then_some(module));
    }
    for (prefix, module) in [("Runtime_", "Core"), ("Editor_", "Editor")] {
        if unit.starts_with(prefix) && modules.contains(module) {
            return Some(Some(module));
        }
    }
    unit.split("ScriptingClasses.")
        .next()
        .filter(|module| unit.contains("ScriptingClasses.") && modules.contains(*module))
        .map(Some)
}

pub(super) fn source_module(path: &str) -> Option<&str> {
    path.split("Modules/")
        .nth(1)?
        .split('/')
        .next()
        .filter(|name| !name.is_empty())
}

pub(super) fn origin(name: &str, kind: &str, path: &str) -> &'static str {
    if matches!(
        kind.to_ascii_lowercase().as_str(),
        "thunk" | "external" | "import"
    ) {
        return "Imports";
    }
    if [
        "android/",
        "androidx/",
        "java/",
        "javax/",
        "kotlin/",
        "com/google/",
        "org/jetbrains/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
    {
        return "Libraries";
    }
    if path.is_empty()
        && [
            "swift_",
            "_swift",
            "objc_",
            "_objc_",
            "__cxa_",
            "__gxx_",
            "__clang_",
            "type metadata",
            "protocol witness",
            "reabstraction thunk",
            "outlined ",
        ]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        return "Runtime";
    }
    "Your code"
}

pub(super) fn file_group(name: &str, path: &str, owner: &str, origin: &str) -> String {
    let symbol = name.split('(').next().unwrap_or(name);
    let owner = if !owner.is_empty() {
        owner
    } else {
        symbol
            .rsplit_once("::")
            .map(|(owner, _)| owner)
            .unwrap_or("")
    };
    let path = if !path.is_empty() {
        path.to_owned()
    } else {
        owner.replace("::", "/")
    };
    let path = path.replace('\\', "/");
    let path = path.trim_matches('/');
    format!(
        "{origin}/{}",
        if path.is_empty() { "Unnamed" } else { path }
    )
}
