use super::{
    app::Prism,
    settings_pages::{model::Path, paths},
};
use crate::error::Result;
use std::path::Path as FilePath;

impl Prism {
    pub(super) fn choose_settings_path(&mut self, key: Path) -> Result<()> {
        let dialog = rfd::FileDialog::new().set_title("Choose a path");
        let picked = if key.is_folder() {
            dialog.pick_folder()
        } else {
            dialog.pick_file()
        };
        if let Some(path) = picked {
            if !path.exists() {
                return Err("The selected path no longer exists".into());
            }
            self.apply_settings_path(key, &path)?;
        }
        Ok(())
    }

    fn apply_settings_path(&mut self, key: Path, path: &FilePath) -> Result<()> {
        let mut selected = self.settings.clone();
        *key.value(&mut selected) = path.to_string_lossy().into_owned();

        match key {
            Path::Rea => {
                selected.tools.rust_ghidra_worker.clear();
                #[cfg(feature = "rea-rust-ghidra")]
                {
                    selected.provider = "ghidra-rust-rea".into();
                }
                crate::analysis::Rea::validate_installation(&selected)?;
            }
            #[cfg(feature = "rea-rust-ghidra")]
            Path::RustGhidraWorker => {
                crate::analysis::Rea::validate_installation(&selected)?;
            }
            _ => {}
        }

        self.settings = selected;
        Ok(())
    }

    pub(super) fn add_custom_agent(&mut self) -> Result<()> {
        let words = shlex::split(&self.settings.conversation.custom_agent)
            .ok_or("Unclosed quote in the agent command")?;
        let (command, arguments) = words
            .split_first()
            .ok_or("Enter an ACP agent command first")?;
        if command.trim().is_empty() {
            return Err("Enter an agent executable before its arguments".into());
        }
        let arguments = shlex::try_join(arguments.iter().map(String::as_str))?;
        self.disconnect_chat_agent()?;

        self.settings.agent = command.clone();
        self.settings.agent_args = arguments;
        self.settings.agent_id.clear();
        self.settings.agent_name.clear();
        Ok(())
    }

    pub(super) fn create_skill(&mut self) -> Result<()> {
        use std::io::Write;
        let folder = paths::skills_dir(&self.settings);
        std::fs::create_dir_all(&folder)?;
        for index in 1..=1000 {
            let name = if index == 1 {
                "new-skill".into()
            } else {
                format!("new-skill-{index}")
            };
            let path = folder.join(format!("{name}.md"));
            let opened = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path);
            let mut file = match opened {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            };
            write!(
                file,
                "---\nname: {name}\ndescription: One line on what this skill does.\n---\n\n1. The first step the agent takes.\n2. The next step.\n"
            )?;
            file.sync_all()?;
            self.playbooks.invalidate();
            return reveal(&path);
        }
        Err("Could not find an unused name in the skills folder".into())
    }

    pub(super) fn clear_recordings(&self) -> Result<()> {
        let folder = crate::settings::support_dir().join("recordings");
        match std::fs::symlink_metadata(&folder) {
            Ok(metadata) if metadata.is_symlink() => {
                Err("The recordings folder is a symbolic link; no files were deleted".into())
            }
            Ok(_) => {
                std::fs::remove_dir_all(&folder)?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

pub(super) fn reveal(path: &FilePath) -> Result<()> {
    if !path.exists() {
        return Err(format!("This path does not exist: {}", path.display()).into());
    }
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .status()?;
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("explorer").arg(path).status()?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let status = std::process::Command::new("xdg-open").arg(path).status()?;
    if !status.success() {
        return Err(format!("Could not reveal {}: {status}", path.display()).into());
    }
    Ok(())
}
