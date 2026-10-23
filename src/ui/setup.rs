use super::app::Prism;
use super::{settings_fields, settings_navigation::SettingsPage, theme::Palette, widgets};
use crate::model::Screen;
use eframe::egui::{self, FontId, Frame, Rect, RichText, Sense};

mod engines;

const STEPS: [&str; 5] = ["Interests", "Tools", "Agent", "Shortcuts", "Try it"];
pub(super) const KEYBOARD_STEP: usize = 3;
const LAST_STEP: usize = STEPS.len() - 1;
const FOOTER_SPACE: f32 = 56.0;

impl Prism {
    pub(super) fn setup_screen(&mut self, ui: &mut egui::Ui) {
        let before = self.settings.clone();
        self.centered_setup(ui);

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
        let preferred_width = if self.settings_ui.setup_step == KEYBOARD_STEP {
            980.0
        } else {
            760.0
        };
        let width = bounds.width().min(preferred_width);
        let height_id = ui.id().with(("setup-height", self.settings_ui.setup_step));
        let height = ui
            .ctx()
            .data(|data| data.get_temp::<f32>(height_id))
            .unwrap_or(600.0);
        let top = bounds.center().y - height.min(bounds.height()) * 0.5;
        let rect = Rect::from_min_size(
            egui::pos2(bounds.center().x - width * 0.5, top),
            egui::vec2(width, bounds.height()),
        );

        let content = ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            self.setup_flow(ui, bounds.height());
        });
        let measured_height = content.response.rect.height();
        if (height - measured_height).abs() > 1.0 {
            ui.ctx()
                .data_mut(|data| data.insert_temp(height_id, measured_height));
            ui.ctx()
                .request_discard("center setup after content size changes");
        }
    }

    fn setup_flow(&mut self, ui: &mut egui::Ui, max_height: f32) {
        let top = ui.cursor().top();
        self.setup_brand(ui);
        ui.add_space(16.0);
        setup_steps(ui, &mut self.settings_ui.setup_step);
        ui.add_space(24.0);
        self.setup_heading(ui);
        ui.add_space(22.0);

        let header_height = ui.cursor().top() - top;
        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
        egui::ScrollArea::vertical()
            .id_salt(("setup-body", self.settings_ui.setup_step))
            .auto_shrink([false, true])
            .max_height((max_height - header_height - FOOTER_SPACE).max(80.0))
            .show(ui, |ui| self.setup_body(ui));
        ui.add_space(20.0);
        self.setup_footer(ui);
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

    fn setup_heading(&self, ui: &mut egui::Ui) {
        let (title, detail) = match self.settings_ui.setup_step {
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
                "Choose an ACP agent. Sign in through its command line, then start a conversation in Prism.",
            ),
            3 => (
                "Keep the shortcuts you know",
                "Use the keyboard to open files, start conversations and move between code views.",
            ),
            _ => (
                "What should we take apart?",
                "Ask your first question, or drop your own app on the box.",
            ),
        };
        let p = Palette::for_context(ui.ctx());
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(title).size(28.0).color(p.text));
            ui.add_space(6.0);
            ui.label(RichText::new(detail).size(13.5).color(p.muted));
        });
    }

    fn setup_body(&mut self, ui: &mut egui::Ui) {
        let step = self.settings_ui.setup_step;
        if step == 0 {
            self.workspace_category_picker(ui, "");
            return;
        }
        if step == LAST_STEP {
            self.composer(ui);
            return;
        }
        if step == KEYBOARD_STEP {
            self.settings_page(ui, SettingsPage::Keyboard, "");
            return;
        }

        Frame::NONE
            .fill(Palette::for_context(ui.ctx()).well)
            .corner_radius(12)
            .inner_margin(20)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 8.0;
                match step {
                    1 => self.setup_engines(ui),
                    2 => {
                        let menus = self.composer_option_menus();
                        self.model_chip(ui, &menus);
                        settings_fields::agent_settings(ui, &mut self.settings);
                    }
                    _ => self.settings_page(ui, SettingsPage::Keyboard, ""),
                }
            });
    }

    fn setup_footer(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        ui.horizontal(|ui| {
            if self.settings_ui.setup_step > 0
                && widgets::chip(ui, None, "Back", false, false).clicked()
            {
                self.settings_ui.setup_step -= 1;
            }
            if widgets::chip_tinted(ui, None, "Set up later", false, false, p.muted).clicked() {
                self.screen = Screen::Home;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let last = self.settings_ui.setup_step == LAST_STEP;
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

fn setup_steps(ui: &mut egui::Ui, current: &mut usize) {
    let p = Palette::for_context(ui.ctx());
    let labels: Vec<_> = STEPS
        .iter()
        .enumerate()
        .map(|(index, label)| format!("{}  {label}", index + 1))
        .collect();
    let width = labels
        .iter()
        .map(|label| widgets::chip_width(ui, label, false, false))
        .sum::<f32>()
        + ui.spacing().item_spacing.x * (labels.len() - 1) as f32;

    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - width) * 0.5).max(0.0));
        for (index, label) in labels.iter().enumerate() {
            let selected = index == *current;
            let color = if selected { p.text } else { p.muted };
            let response = widgets::chip_tinted(ui, None, label, selected, false, color);
            if response.clicked() {
                *current = index;
            }
        }
    });
}
