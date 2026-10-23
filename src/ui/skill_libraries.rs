use super::{app::Prism, notifications::Level, playbooks, theme::Palette};
use crate::error::Result;
use crate::settings::{research::Category, skill_library::SkillLibrary};
use eframe::egui::{self, RichText};
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

    pub(super) fn skill_settings(&mut self, ui: &mut egui::Ui, query: &str) {
        self.skill_library_controls(ui);
        ui.add_space(16.0);
        self.installed_skill_libraries(ui);
        ui.add_space(16.0);
        self.skill_browser(ui, query);
    }

    fn skill_library_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Download a skill library").strong());
        ui.label("Enter a public GitHub repository. Prism downloads skill Markdown and license files on demand. Git must be installed.");
        ui.add(
            egui::TextEdit::singleline(&mut self.settings_ui.libraries.repository)
                .hint_text("https://github.com/owner/repository")
                .desired_width(ui.available_width())
                .char_limit(2048),
        );
        let busy = self.settings_ui.libraries.pending.is_some();
        let mut download = false;
        ui.horizontal_wrapped(|ui| {
            download = ui
                .add_enabled(
                    !busy && !self.settings_ui.libraries.repository.trim().is_empty(),
                    egui::Button::new("Download from GitHub"),
                )
                .clicked();
            if ui
                .add_enabled(!busy, egui::Button::new("Import local folder…"))
                .clicked()
            {
                self.import_skill_folder(ui.ctx());
            }
            if busy {
                ui.spinner();
                ui.label("Updating library…");
                if ui.button("Cancel").clicked() {
                    self.settings_ui
                        .libraries
                        .cancelled
                        .store(true, Ordering::Relaxed);
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Suggested:");
            if ui.link("mcp-gamehacking").clicked() {
                self.settings_ui.libraries.repository = playbooks::download::GAME_LIBRARY.into();
            }
            ui.hyperlink_to("View repository", playbooks::download::GAME_LIBRARY);
        });
        ui.label(RichText::new("Libraries stay outside the app bundle. Adding one does not run its scripts or start an MCP server. Source archives are not downloaded.").small().color(Palette::for_context(ui.ctx()).muted));
        if download {
            self.download_skill_library(ui.ctx());
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

    fn installed_skill_libraries(&mut self, ui: &mut egui::Ui) {
        let busy = self.settings_ui.libraries.pending.is_some();
        let mut removal = None;
        for library in &mut self.settings.desktop.skill_libraries {
            if library_row(ui, library, busy) {
                removal = Some(library.clone());
            }
        }
        if let Some(library) = removal {
            self.remove_skill_library(ui.ctx(), library);
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

fn library_row(ui: &mut egui::Ui, library: &mut SkillLibrary, busy: bool) -> bool {
    let mut remove = false;
    ui.push_id(library.path.clone(), |ui| {
        let title = library
            .repository
            .as_deref()
            .unwrap_or_else(|| library.path.to_str().unwrap_or("Local library"))
            .to_owned();
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut library.enabled, title);
            let label = if library.repository.is_some() {
                "Remove download"
            } else {
                "Disconnect"
            };
            remove = ui.add_enabled(!busy, egui::Button::new(label)).clicked();
        });
        ui.checkbox(
            &mut library.include_archives,
            "Include archived-project skill summaries",
        );
        if let Some(revision) = &library.revision {
            ui.label(RichText::new(format!("Revision {revision}")).small().weak());
        }
        egui::CollapsingHeader::new("Categories for skills without topic metadata").show(
            ui,
            |ui| {
                for category in Category::ALL {
                    let mut selected = library.fallback_categories.contains(&category);
                    if ui.checkbox(&mut selected, category.label()).changed() {
                        library
                            .fallback_categories
                            .retain(|entry| *entry != category);
                        if selected {
                            library.fallback_categories.push(category);
                        }
                    }
                }
            },
        );
        ui.add_space(8.0);
    });
    remove
}
