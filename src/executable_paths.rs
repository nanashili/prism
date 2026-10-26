use std::path::{Path, PathBuf};

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
        for relative in [
            ".cargo/bin",
            ".local/bin",
            ".bun/bin",
            ".npm-global/bin",
            ".volta/bin",
            ".local/share/pnpm",
            ".opencode/bin",
            ".nvm/current/bin",
        ] {
            let path = home.join(relative);
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths
}

pub(crate) fn find(name: &str) -> String {
    if let Some(path) = locate(name, &search_path()) {
        return path.to_string_lossy().into_owned();
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

pub(crate) fn locate(name: &str, directories: &[PathBuf]) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let command = Path::new(name);
    if command.is_absolute() || command.components().count() > 1 {
        return is_executable(command).then(|| command.to_owned());
    }
    for directory in directories {
        let path = directory.join(command);
        if is_executable(&path) {
            return Some(path);
        }
        #[cfg(windows)]
        if command.extension().is_none() {
            let extensions =
                std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
            for extension in extensions.split(';').filter(|value| !value.is_empty()) {
                let candidate = directory.join(format!("{name}{extension}"));
                if is_executable(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub(crate) fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub(crate) fn codex_agent() -> (String, String) {
    let installed = find("codex-acp");
    if std::path::Path::new(&installed).is_file() {
        (installed, String::new())
    } else {
        (find("npx"), "--yes @agentclientprotocol/codex-acp".into())
    }
}
