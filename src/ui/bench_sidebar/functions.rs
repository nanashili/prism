use super::{parts, state::LoadedPage};
use crate::storage::navigation::{Page, Row, Sort};
use crate::ui::{app::Prism, icons::Icon, popup, theme::Palette, widgets};
use crate::worker::Job;
use eframe::egui::{self, FontId, Frame};
use std::time::{Duration, Instant};

impl Prism {
    pub(super) fn ensure_sidebar(&mut self, ui: &egui::Ui) {
        if self.workspace.navigation.pages.contains_key("") {
            return;
        }
        if let Some(deadline) = self.workspace.navigation.debounce
            && Instant::now() < deadline
        {
            ui.ctx()
                .request_repaint_after(deadline.saturating_duration_since(Instant::now()));
            return;
        }
        self.request_sidebar_page(String::new(), 0);
    }

    pub(super) fn request_sidebar_page(&mut self, parent: String, offset: usize) {
        let Some(project) = &self.workspace.project else {
            return;
        };
        let mut query = self.workspace.navigation.query.clone();
        query.parent = parent.clone();
        query.offset = offset;
        query.filter = self.workspace.filter.clone();
        query.renamed = project.renamed.clone();
        let job = Job::Sidebar {
            project: project.target.sha256.clone(),
            generation: self.workspace.navigation.generation,
            query,
        };
        match self.submit(job) {
            Ok(()) => {
                if self
                    .workspace
                    .navigation
                    .pages
                    .get(&parent)
                    .is_some_and(|loaded| loaded.offset != offset)
                {
                    self.workspace.navigation.discard_children(&parent);
                }
                self.workspace.navigation.pages.insert(
                    parent,
                    LoadedPage {
                        page: Page::default(),
                        offset,
                        ready: false,
                        error: None,
                    },
                );
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(in crate::ui) fn receive_sidebar(
        &mut self,
        project: &str,
        generation: u64,
        parent: String,
        offset: usize,
        page: std::result::Result<Page, String>,
    ) {
        if self
            .workspace
            .project
            .as_ref()
            .is_some_and(|current| current.target.sha256 == project)
            && self.workspace.navigation.generation == generation
            && self
                .workspace
                .navigation
                .pages
                .get(&parent)
                .is_some_and(|loaded| loaded.offset == offset)
        {
            let (page, error) = match page {
                Ok(page) => (page, None),
                Err(error) => (Page::default(), Some(error)),
            };
            self.workspace.navigation.pages.insert(
                parent,
                LoadedPage {
                    page,
                    offset,
                    ready: true,
                    error,
                },
            );
        }
    }

    pub(super) fn function_filter(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        Frame::NONE
            .fill(p.fill)
            .corner_radius(7)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Icon::Search.show(ui, 12.0, p.muted);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.workspace.filter)
                            .frame(false)
                            .hint_text("Filter functions or modules")
                            .char_limit(256)
                            .id(egui::Id::new("function-filter"))
                            .margin(0.0)
                            .desired_width((ui.available_width() - 26.0).max(20.0))
                            .font(FontId::proportional(12.5)),
                    );
                    if response.changed() {
                        self.workspace.navigation.invalidate();
                        self.workspace.navigation.debounce =
                            Some(Instant::now() + Duration::from_millis(180));
                    }
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        self.submit_function_filter();
                    }
                    let menu = widgets::icon_button(ui, Icon::Sort, "Filter and group", 22.0);
                    popup::menu(&menu, 236.0).show(|ui| self.function_filter_menu(ui));
                });
            });
        if self.workspace.navigation.query.sort == Sort::Modules {
            ui.label(
                egui::RichText::new("Modules · ≈ inferred · Unmapped if unknown")
                    .size(11.0)
                    .color(p.faint),
            );
        }
        if !self.workspace.filter.trim().is_empty()
            && let Some(loaded) = self.workspace.navigation.pages.get("")
            && loaded.ready
        {
            ui.label(
                egui::RichText::new(format!(
                    "{} matches. ↩ opens the first.",
                    parts::number(loaded.page.matched)
                ))
                .size(11.0)
                .color(p.faint),
            );
        }
    }

    fn submit_function_filter(&mut self) {
        let Some(loaded) = self.workspace.navigation.pages.get("") else {
            return;
        };
        if !loaded.ready || loaded.error.is_some() {
            return;
        }
        let first = loaded.page.rows.iter().find_map(|row| match row {
            Row::Function { procedure, .. } => Some(procedure.clone()),
            _ => None,
        });
        if let Some(procedure) = first {
            self.preview_function(
                &procedure.address,
                &procedure.name,
                crate::model::View::Pseudocode,
            );
        } else if !self.workspace.filter.trim().is_empty() && self.pending == 0 && !self.chat.busy {
            self.chat.draft = format!(
                "Find the functions that have to do with {:?} in this project.",
                self.workspace.filter.trim()
            );
            self.workspace.agent_closed = false;
            self.send_prompt();
        }
    }
}
