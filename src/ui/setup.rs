use super::app::Prism;
use super::{settings_navigation::SettingsPage, theme::Palette, widgets};
use crate::model::Screen;
use crate::ui::presentation::{Border, Caret, Presence};
use eframe::egui::{self, FontId, Rect, RichText, Sense};

mod agents;
mod engines;
mod transition;
mod try_it;

const STEPS: [&str; 5] = ["Interests", "Tools", "Agent", "Shortcuts", "Try it"];
pub(super) const KEYBOARD_STEP: usize = 3;
const LAST_STEP: usize = STEPS.len() - 1;
const FOOTER_SPACE: f32 = 56.0;
const FLOW_HEIGHT: f32 = 760.0;

impl Prism {
    pub(super) fn setup_screen(&mut self, ui: &mut egui::Ui) {
        let before = self.settings.clone();
        let previous_step = self.settings_ui.setup_step;
        self.centered_setup(ui);

        if self.settings_ui.setup_step != previous_step {
            self.settings_ui.keyboard.recording = None;
            egui::Popup::close_all(ui.ctx());
            ui.ctx().request_repaint();
        }
        if self.screen != Screen::Setup {
            self.settings.research.setup_complete = true;
        }
        if self.settings != before {
            self.queue_settings_save(ui.ctx());
        }
    }

    fn centered_setup(&mut self, ui: &mut egui::Ui) {
        let bounds = ui
            .available_rect_before_wrap()
            .shrink2(egui::vec2(24.0, 24.0));
        let rect = Rect::from_center_size(
            bounds.center(),
            egui::vec2(bounds.width().min(980.0), bounds.height().min(FLOW_HEIGHT)),
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            self.setup_flow(ui);
        });
    }

    fn setup_flow(&mut self, ui: &mut egui::Ui) {
        let bounds = ui.max_rect();
        self.setup_brand(ui);
        ui.add_space(16.0);
        setup_steps(ui, &mut self.settings_ui.setup_step);
        ui.add_space(24.0);

        let page = Rect::from_min_max(
            egui::pos2(bounds.left(), ui.cursor().top()),
            egui::pos2(bounds.right(), bounds.bottom() - FOOTER_SPACE),
        );
        let duration = transition::duration(ui);
        transition::show(
            ui,
            page,
            self.settings_ui.setup_step,
            duration,
            |ui, step| {
                self.setup_page(ui, step);
            },
        );

        let footer = Rect::from_min_max(
            egui::pos2(bounds.left(), bounds.bottom() - 36.0),
            bounds.max,
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(centered_width(footer, 760.0)),
            |ui| self.setup_footer(ui),
        );
    }

    fn setup_page(&mut self, ui: &mut egui::Ui, step: usize) {
        let width = if step == KEYBOARD_STEP { 980.0 } else { 760.0 };
        let rect = centered_width(ui.max_rect(), width);
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            self.setup_heading(ui, step);
            ui.add_space(22.0);
            self.setup_scroll_body(ui, step);
        });
    }

    fn setup_scroll_body(&mut self, ui: &mut egui::Ui, step: usize) {
        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
        egui::ScrollArea::vertical()
            .id_salt(("setup-body", step))
            .auto_shrink([false, true])
            .max_height(ui.available_height().max(80.0))
            .show(ui, |ui| self.setup_body(ui, step));
    }

    fn setup_brand(&self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let text = ui
            .painter()
            .layout_no_wrap("Prism".into(), FontId::proportional(17.0), p.text);
        let width = 32.0 + 10.0 + text.size().x;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::hover());
        let left = rect.center().x - width * 0.5;
        let logo = Rect::from_min_size(egui::pos2(left, rect.top()), egui::vec2(32.0, 32.0));
        egui::Image::new(super::branding::LOGO).paint_at(ui, logo);
        ui.painter().galley(
            egui::pos2(left + 42.0, rect.center().y - text.size().y * 0.5),
            text,
            p.text,
        );
    }

    fn setup_heading(&self, ui: &mut egui::Ui, step: usize) {
        let (title, detail) = match step {
            0 => (
                "What will you reverse engineer?",
                "Pick your interests. You can change them any time in Settings.",
            ),
            1 => (
                "Tools for your workspace",
                "Configure the tools for your selected categories. You can add more later in Settings.",
            ),
            2 => (
                "Pick your agent",
                "Choose an agent found on your computer. You can change it later.",
            ),
            3 => (
                "Keep the shortcuts you know",
                "Use the keyboard to open files, start conversations and move between code views.",
            ),
            _ => (
                "Start your first investigation",
                "Open something to explore, or ask your agent where to begin.",
            ),
        };
        let p = Palette::for_context(ui.ctx());
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(title).size(28.0).color(p.text));
            ui.add_space(6.0);
            ui.label(RichText::new(detail).size(13.5).color(p.muted));
        });
    }

    fn setup_body(&mut self, ui: &mut egui::Ui, step: usize) {
        match step {
            0 => self.workspace_category_picker(ui, ""),
            1 => self.setup_engines(ui),
            2 => self.setup_agents(ui),
            KEYBOARD_STEP => self.settings_page(ui, SettingsPage::Keyboard, ""),
            _ => self.setup_try_it(ui),
        }
    }

    fn setup_footer(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        ui.horizontal(|ui| {
            if ui
                .add_enabled_ui(self.settings_ui.setup_step > 0, |ui| {
                    widgets::chip(ui, None, "Back", Border::Hidden, Caret::Hidden)
                })
                .inner
                .clicked()
            {
                self.settings_ui.setup_step -= 1;
            }
            let last = self.settings_ui.setup_step == LAST_STEP;
            if !last
                && widgets::chip_tinted(
                    ui,
                    None,
                    "Set up later",
                    Border::Hidden,
                    Caret::Hidden,
                    p.muted,
                )
                .clicked()
            {
                self.screen = Screen::Home;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if last { "Open Prism" } else { "Continue" };
                if ui
                    .add_sized(
                        [110.0, 36.0],
                        egui::Button::new(RichText::new(label).color(p.card))
                            .fill(p.text)
                            .corner_radius(8),
                    )
                    .clicked()
                {
                    if last {
                        self.screen = Screen::Home;
                    } else {
                        self.settings_ui.setup_step += 1;
                    }
                }
            });
        });
    }
}

fn centered_width(rect: Rect, width: f32) -> Rect {
    Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width().min(width), rect.height()),
    )
}

fn setup_steps(ui: &mut egui::Ui, current: &mut usize) {
    let p = Palette::for_context(ui.ctx());
    let labels: Vec<_> = STEPS
        .iter()
        .enumerate()
        .map(|(index, label)| format!("{}  {label}", index + 1))
        .collect();
    let width = labels
        .iter()
        .map(|label| widgets::chip_width(ui, label, Presence::Absent, Caret::Hidden))
        .sum::<f32>()
        + ui.spacing().item_spacing.x * (labels.len() - 1) as f32;

    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - width) * 0.5).max(0.0));
        for (index, label) in labels.iter().enumerate() {
            let selected = index == *current;
            let color = if selected { p.text } else { p.muted };
            let response = widgets::chip_tinted(
                ui,
                None,
                label,
                Border::from(selected),
                Caret::Hidden,
                color,
            );
            if response.clicked() {
                *current = index;
            }
        }
    });
}
