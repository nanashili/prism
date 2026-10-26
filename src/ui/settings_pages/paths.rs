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
        Path::RustGhidraWorker => true,
        Path::Rea => true,
        _ => false,
    };
    let use_automatic = value.is_empty() || (matches!(key, Path::Rea) && value == "rea");
    let shown = if use_automatic {
        if automatic { "Automatic" } else { "Not found" }.into()
    } else {
        display(&path)
    };
    let mut row = Row::new(title, shown).button("Choose…", Command::Choose(key));
    row.mono = true;
    if !use_automatic {
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
            #[cfg(feature = "rea-rust-ghidra")]
            Self::RustGhidraWorker => &mut settings.tools.rust_ghidra_worker,
            Self::Hopper => &mut settings.tools.hopper_path,
            Self::Ida => &mut settings.tools.ida_path,
            Self::Jadx => &mut settings.jadx,
        }
    }

    pub(in crate::ui) fn is_folder(self) -> bool {
        matches!(self, Self::Exports | Self::Skills)
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
