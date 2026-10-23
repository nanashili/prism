use super::model::{Command, Path, Row};
use crate::settings::Settings;
use std::path::{Path as FilePath, PathBuf};

pub(in crate::ui) fn display(path: &FilePath) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(relative) = path.strip_prefix(home)
    {
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}

pub(super) fn row(title: &str, value: &str, key: Path) -> Row<'static> {
    let path = PathBuf::from(value);
    let automatic = match key {
        #[cfg(feature = "rea-rust-ghidra")]
        Path::RustRea | Path::RustGhidraWorker => true,
        Path::ReaNode => true,
        _ => false,
    };
    let shown = if value.is_empty() {
        if automatic { "Automatic" } else { "Not found" }.into()
    } else {
        display(&path)
    };
    let mut row = Row::new(title, shown).button("Choose…", Command::Choose(key));
    row.mono = true;
    if !value.is_empty() {
        row = row.button("Show in Finder", Command::Reveal(path));
        if automatic {
            row = row.button("Use automatic", Command::ResetPath(key));
        }
    }
    row
}

impl Path {
    pub(in crate::ui) fn value(self, settings: &mut Settings) -> &mut String {
        match self {
            Self::Exports => &mut settings.desktop.export_dir,
            Self::Skills => &mut settings.desktop.skills_dir,
            Self::Debugger => &mut settings.lldb,
            Self::Rizin => &mut settings.rizin,
            Self::Rea => &mut settings.rea,
            #[cfg(feature = "prism-ghidra-rea")]
            Self::Ghidra => &mut settings.tools.ghidra_dir,
            #[cfg(feature = "prism-ghidra-rea")]
            Self::Java => &mut settings.tools.java_home,
            Self::ReaNode => &mut settings.tools.rea_node,
            #[cfg(feature = "rea-rust-ghidra")]
            Self::RustRea => &mut settings.tools.rust_rea,
            #[cfg(feature = "rea-rust-ghidra")]
            Self::RustGhidraWorker => &mut settings.tools.rust_ghidra_worker,
            Self::Hopper => &mut settings.tools.hopper_path,
            Self::Ida => &mut settings.tools.ida_path,
            Self::Jadx => &mut settings.jadx,
        }
    }

    pub(in crate::ui) fn is_folder(self) -> bool {
        match self {
            Self::Exports | Self::Skills => true,
            #[cfg(feature = "prism-ghidra-rea")]
            Self::Ghidra | Self::Java => true,
            _ => false,
        }
    }
}

pub(in crate::ui) fn skills_dir(settings: &Settings) -> PathBuf {
    if settings.desktop.skills_dir.is_empty() {
        crate::settings::support_dir().join("skills")
    } else {
        PathBuf::from(&settings.desktop.skills_dir)
    }
}

pub(in crate::ui) fn exports_dir(settings: &Settings) -> PathBuf {
    if settings.desktop.export_dir.is_empty() {
        dirs::download_dir()
            .unwrap_or_else(crate::settings::support_dir)
            .join("Prism")
    } else {
        PathBuf::from(&settings.desktop.export_dir)
    }
}
