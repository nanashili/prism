use crate::ui::presentation::Selection;
pub(super) mod history;
pub(super) mod inspector;
mod tabs;

use super::app::Prism;
use super::{icons::Icon, theme::Palette, widgets};
use eframe::egui::{self, Frame};

impl Prism {
    pub(super) fn agent_dock(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("agent-dock-header")
            .exact_height(super::tab_strip::HEIGHT)
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show_inside(ui, |ui| self.agent_dock_header(ui));
        if self.chat_tabs.inspector {
            self.dock_inspector(ui);
        } else {
            ui.push_id(self.chat_tabs.active, |ui| self.conversation(ui));
        }
    }

    fn agent_dock_header(&mut self, ui: &mut egui::Ui) {
        let controls = self.agent_chat_tabs(ui);
        let mut buttons =
            ui.new_child(egui::UiBuilder::new().max_rect(controls.shrink2(egui::vec2(4.0, 4.0))));
        buttons.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            self.past_chats_button(ui);
            if widgets::icon_button(ui, Icon::NewChat, "New chat", 28.0).clicked() {
                self.new_chat();
            }
            let inspector = self.chat_tabs.inspector;
            let response = widgets::icon_button(
                ui,
                Icon::Info,
                if inspector {
                    "Back to the agent"
                } else {
                    "Inspector"
                },
                28.0,
            );
            if inspector {
                ui.painter()
                    .rect_filled(response.rect, 7.0, Palette::for_context(ui.ctx()).sel);
                Icon::Info.paint(
                    ui,
                    response.rect.shrink(6.0),
                    Palette::for_context(ui.ctx()).text,
                );
            }
            if response.clicked() {
                self.chat_tabs.inspector = !inspector;
            }
            if widgets::icon_button(ui, Icon::Close, "Close panel", 28.0).clicked() {
                self.workspace.agent_closed = true;
            }
        });
    }

    pub(super) fn empty_agent(&mut self, ui: &mut egui::Ui) {
        let playbooks: Vec<_> = self
            .playbooks
            .enabled(&self.settings)
            .into_iter()
            .take(8)
            .collect();
        let playbook_height = playbooks.len() as f32 * 36.0;
        let height = 64.0 + 10.0 + 24.0 + 10.0 + 18.0 + 20.0 + 20.0 + playbook_height;
        let top = ((ui.available_height() - height) * 0.5).max(10.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(top);
                Frame::NONE
                    .inner_margin(egui::Margin::symmetric(8, 0))
                    .show(ui, |ui| {
                        let p = Palette::for_context(ui.ctx());
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Image::new(crate::ui::branding::LOGO)
                                    .fit_to_exact_size(egui::vec2(64.0, 64.0)),
                            );
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("What do you want to dig into?").size(18.0),
                            );
                            ui.add_space(10.0);
                            if let Some(project) = &self.workspace.project {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} · {} functions shown",
                                        project.target.name,
                                        self.workspace.rows.len()
                                    ))
                                    .size(12.0)
                                    .color(p.muted),
                                );
                            }
                        });
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("Playbooks").size(11.5).color(p.faint));
                        if ui.link("Browse all skills").clicked() {
                            self.open_settings_page(
                                super::settings_navigation::SettingsPage::Skills,
                            );
                        }
                        for playbook in playbooks {
                            if widgets::row(
                                ui,
                                Icon::Bolt,
                                &playbook.title,
                                Selection::Unselected,
                                36.0,
                            )
                            .clicked()
                            {
                                self.run_playbook(&playbook);
                            }
                        }
                    });
            });
    }
}
