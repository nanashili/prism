mod settings;

use super::{app::Prism, notifications::Level, playbooks};
use crate::error::Result;
use crate::settings::{research::Category, skill_library::SkillLibrary};
use eframe::egui;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
};

enum Change {
    Add(SkillLibrary),
    Remove(PathBuf),
}

pub(super) struct LibraryUi {
    pub repository: String,
    pub query: String,
    pub show_all: bool,
    pending: Option<Receiver<Result<Change>>>,
    cancelled: Arc<AtomicBool>,
}

impl Default for LibraryUi {
    fn default() -> Self {
        Self {
            repository: String::new(),
            query: String::new(),
            show_all: false,
            pending: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for LibraryUi {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl LibraryUi {
    fn start(
        &mut self,
        context: &egui::Context,
        operation: impl FnOnce(&AtomicBool) -> Result<Change> + Send + 'static,
    ) -> Result<()> {
        if self.pending.is_some() {
            return Err("A skill library operation is already running".into());
        }
        let (sender, receiver) = mpsc::channel();
        let context = context.clone();
        self.cancelled = Arc::new(AtomicBool::new(false));
        let cancelled = self.cancelled.clone();
        std::thread::Builder::new()
            .name("skill-library".into())
            .spawn(move || {
                let result = operation(&cancelled);
                if sender.send(result).is_ok() {
                    context.request_repaint();
                }
            })?;
        self.pending = Some(receiver);
        Ok(())
    }
}

impl Prism {
    pub(super) fn poll_skill_libraries(&mut self, context: &egui::Context) {
        self.playbooks.poll();
        if self.playbooks.loading() {
            context.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let Some(receiver) = &self.settings_ui.libraries.pending else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("The skill library worker stopped. Try again.".into())
            }
        };
        self.settings_ui.libraries.pending = None;
        match result {
            Ok(Change::Add(library)) => {
                let libraries = &mut self.settings.desktop.skill_libraries;
                if !libraries.iter().any(|entry| entry.path == library.path) {
                    libraries.push(library);
                }
                self.playbooks.invalidate();
                self.queue_settings_save(context);
                self.notify(Level::Success, "Skill library added", "", "");
            }
            Ok(Change::Remove(path)) => {
                self.settings
                    .desktop
                    .skill_libraries
                    .retain(|library| library.path != path);
                self.playbooks.invalidate();
                self.queue_settings_save(context);
                self.notify(Level::Success, "Skill download removed", "", "");
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn download_skill_library(&mut self, context: &egui::Context) {
        let url = self.settings_ui.libraries.repository.clone();
        let fallback = self.settings.research.categories.clone();
        let result = self.settings_ui.libraries.start(context, move |cancelled| {
            let mut library = playbooks::download::install(&url, &download_dir(), cancelled)?;
            library.fallback_categories =
                fallback_categories(&library.path, library.repository.as_deref(), fallback);
            Ok(Change::Add(library))
        });
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
    }

    fn import_skill_folder(&mut self, context: &egui::Context) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose a repository or skills folder")
            .pick_folder()
        else {
            return;
        };
        let fallback = self.settings.research.categories.clone();
        let result = self.settings_ui.libraries.start(context, move |cancelled| {
            playbooks::download::check_cancelled(cancelled)?;
            let path = playbooks::library::root(&folder)?;
            let library = SkillLibrary {
                fallback_categories: fallback_categories(&folder, None, fallback),
                path,
                ..Default::default()
            };
            let index = playbooks::library::index(
                &folder.join(".no-custom-skills"),
                std::slice::from_ref(&library),
            );
            if index.entries.is_empty() {
                return Err(crate::error::failure(format!(
                    "The folder could not be imported. {}",
                    index.warnings.join("; ")
                )));
            }
            playbooks::download::check_cancelled(cancelled)?;
            Ok(Change::Add(library))
        });
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
    }

    fn remove_skill_library(&mut self, context: &egui::Context, library: SkillLibrary) {
        if library.repository.is_none() {
            self.settings
                .desktop
                .skill_libraries
                .retain(|entry| entry.path != library.path);
            self.playbooks.invalidate();
            return;
        }
        let result = self.settings_ui.libraries.start(context, move |cancelled| {
            playbooks::download::check_cancelled(cancelled)?;
            playbooks::download::remove(&library, &download_dir())?;
            Ok(Change::Remove(library.path))
        });
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
    }
}

fn download_dir() -> PathBuf {
    crate::settings::support_dir().join("skill-libraries")
}

fn fallback_categories(
    path: &std::path::Path,
    repository: Option<&str>,
    current: Vec<Category>,
) -> Vec<Category> {
    if repository.is_some_and(|url| url.eq_ignore_ascii_case(playbooks::download::GAME_LIBRARY))
        || path
            .file_name()
            .is_some_and(|name| name == "mcp-gamehacking")
    {
        vec![
            Category::GameEngines,
            Category::GameRuntime,
            Category::GameSecurity,
        ]
    } else {
        current
    }
}
