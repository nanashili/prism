use std::path::PathBuf;

pub(crate) fn search_path() -> Vec<PathBuf> {
    let mut paths: Vec<_> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    for path in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        let path = PathBuf::from(path);
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    if let Some(home) = dirs::home_dir() {
        for relative in [".cargo/bin", ".local/bin"] {
            let path = home.join(relative);
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths
}

pub(crate) fn find(name: &str) -> String {
    for directory in search_path() {
        let path = directory.join(name);
        if path.is_file() {
            return path.to_string_lossy().into_owned();
        }
    }
    #[cfg(target_os = "macos")]
    if name == "lldb-dap" {
        let path = "/Applications/Xcode.app/Contents/Developer/usr/bin/lldb-dap";
        if std::path::Path::new(path).is_file() {
            return path.into();
        }
    }
    #[cfg(target_os = "macos")]
    if name == "rizin" {
        let path = "/Applications/Prism.app/Contents/Resources/rizin/bin/rizin";
        if std::path::Path::new(path).is_file() {
            return path.into();
        }
    }
    name.into()
}

pub(crate) fn codex_agent() -> (String, String) {
    let installed = find("codex-acp");
    if std::path::Path::new(&installed).is_file() {
        (installed, String::new())
    } else {
        (find("npx"), "--yes @agentclientprotocol/codex-acp".into())
    }
}
