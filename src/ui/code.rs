use super::{app::Prism, syntax, theme::Palette};
use crate::settings::Settings;
use eframe::egui::{self, FontId, RichText};

impl Prism {
    pub(super) fn code(&mut self, ui: &mut egui::Ui) {
        if self.workspace.text.is_empty() {
            ui.add_space(24.0);
            ui.label(
                RichText::new(if self.pending > 0 {
                    "Reading…"
                } else {
                    "Select a function or a view."
                })
                .weak(),
            );
            return;
        }
        let font = font(ui.ctx(), &self.settings);
        egui::ScrollArea::both()
            .id_salt(("code", self.workspace.view, &self.workspace.selected))
            .auto_shrink([false, false])
            .show_rows(
                ui,
                line_height(&self.settings),
                self.workspace.lines.len(),
                |ui, range| {
                    for index in range {
                        let source = &self.workspace.text[self.workspace.lines[index].clone()];
                        let text = display_line(source, self.workspace.view, &self.settings);
                        line(
                            ui,
                            index,
                            text.trim_end_matches('\n'),
                            &self.settings,
                            &font,
                        );
                    }
                },
            );
    }
}

pub(super) fn font(context: &egui::Context, settings: &Settings) -> FontId {
    FontId::new(
        settings.code_size,
        super::code_fonts::family(context, &settings.code_font),
    )
}

pub(super) fn line_height(settings: &Settings) -> f32 {
    let spacing = match settings.line_spacing.as_str() {
        "Compact" => 1.35,
        "Relaxed" => 1.85,
        _ => 1.6,
    };
    (settings.code_size * spacing).round()
}

pub(super) fn line(
    ui: &mut egui::Ui,
    index: usize,
    text: &str,
    settings: &Settings,
    font: &FontId,
) {
    let palette = Palette::for_context(ui.ctx());
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), line_height(settings)),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            if settings.line_numbers {
                ui.add_sized(
                    [28.0, line_height(settings)],
                    egui::Label::new(
                        RichText::new(format!("{:>4}", index + 1))
                            .font(font.clone())
                            .color(palette.faint),
                    ),
                );
                ui.add_space(8.0);
            }
            ui.add(
                egui::Label::new(syntax::highlight(
                    text,
                    font.clone(),
                    ui.visuals().dark_mode,
                    &settings.color_scheme,
                ))
                .selectable(true)
                .extend(),
            );
        },
    );
}

fn display_line<'a>(
    text: &'a str,
    view: crate::model::View,
    settings: &Settings,
) -> std::borrow::Cow<'a, str> {
    if !matches!(view, crate::model::View::Assembly | crate::model::View::Hex) {
        return text.into();
    }
    let trimmed = text.trim_start();
    let token = trimmed.split_whitespace().next().unwrap_or("");
    let address = token.trim_end_matches(':');
    let digits = address.strip_prefix("0x").unwrap_or(address);
    if digits.len() < 4 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return text.into();
    }
    let prefix = text.len() - trimmed.len();
    format!(
        "{}{}{}{}",
        &text[..prefix],
        settings.views.address(address),
        if token.ends_with(':') { ":" } else { "" },
        &trimmed[token.len()..]
    )
    .into()
}
