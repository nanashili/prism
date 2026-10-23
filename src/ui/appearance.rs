use super::{code_fonts, settings_controls as control, settings_rows as rows};
use crate::settings::Settings;
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings) {
    super::settings_fields::theme_cards(ui, &mut settings.appearance);
    ui.add_space(26.0);
    rows::group(ui, "Window", |ui| {
        interface_size(ui, settings);
        rows::row(
            ui,
            false,
            "Transparency",
            "How much of your desktop shows through around the edges. All the way down is solid.",
            212.0,
            |ui| {
                control::range(
                    ui,
                    "Transparency",
                    &mut settings.transparency,
                    0.0..=100.0,
                    5.0,
                    "%",
                );
            },
        );
        rows::row(
            ui,
            false,
            "Frost",
            "How much the desktop behind the window is blurred.",
            212.0,
            |ui| {
                let response = ui.add_enabled_ui(false, |ui| {
                    control::range(ui, "Frost", &mut 60.0, 0.0..=100.0, 5.0, "%");
                });
                response.response.on_hover_text(
                    "Blur intensity is controlled by the operating system's native window material.",
                );
            },
        );
    });
    super::settings_preview::window(ui, settings.transparency);
    ui.add_space(26.0);
    code_settings(ui, settings);
}

fn interface_size(ui: &mut egui::Ui, settings: &mut Settings) {
    let options = ["Small", "Default", "Large"];
    let mut choice = if settings.ui_scale < 0.96 {
        "Small"
    } else if settings.ui_scale > 1.05 {
        "Large"
    } else {
        "Default"
    }
    .to_owned();
    let before = choice.clone();
    let width = control::segmented_width(ui, &options);
    rows::row(
        ui,
        true,
        "Interface size",
        "Text and controls outside the code.",
        width,
        |ui| {
            control::segmented(ui, &mut choice, &options);
        },
    );
    if choice != before {
        settings.ui_scale = match choice.as_str() {
            "Small" => 0.92,
            "Large" => 1.1,
            _ => 1.0,
        };
    }
}

pub(super) fn code_settings(ui: &mut egui::Ui, settings: &mut Settings) {
    rows::group(ui, "Code", |ui| {
        let fonts = code_fonts::available(ui.ctx());
        let fonts: Vec<_> = fonts.iter().map(String::as_str).collect();
        rows::row(
            ui,
            true,
            "Code font",
            "Used for pseudocode, assembly, hex and code in replies.",
            150.0,
            |ui| {
                control::select(ui, &mut settings.code_font, &fonts);
            },
        );
        rows::row(
            ui,
            false,
            "Code size",
            "Applies to every code view and the graph.",
            212.0,
            |ui| {
                control::range(
                    ui,
                    "Code size",
                    &mut settings.code_size,
                    10.0..=18.0,
                    1.0,
                    " pt",
                );
            },
        );
        let options = ["Compact", "Normal", "Relaxed"];
        let width = control::segmented_width(ui, &options);
        rows::row(ui, false, "Line spacing", "", width, |ui| {
            control::segmented(ui, &mut settings.line_spacing, &options);
        });
        rows::row(
            ui,
            false,
            "Colours",
            "How keywords, names, strings and numbers are coloured.",
            150.0,
            |ui| {
                control::select(
                    ui,
                    &mut settings.color_scheme,
                    &["Prism", "Classic", "Quiet"],
                );
            },
        );
        rows::row(
            ui,
            false,
            "Line numbers",
            "In pseudocode and shader views.",
            34.0,
            |ui| {
                control::toggle(ui, "Line numbers", &mut settings.line_numbers);
            },
        );
        rows::row(
            ui,
            false,
            "Use the code size in graphs",
            "Off keeps graph blocks small so more fits on screen.",
            34.0,
            |ui| {
                control::toggle(
                    ui,
                    "Use the code size in graphs",
                    &mut settings.graph_uses_code_size,
                );
            },
        );
    });
    ui.add_space(12.0);
    code_preview(ui, settings);
}

fn code_preview(ui: &mut egui::Ui, settings: &Settings) {
    egui::Frame::NONE
        .fill(super::theme::Palette::for_context(ui.ctx()).well)
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let font = super::code::font(ui.ctx(), settings);
            for (index, text) in [
                "int check_license(char *key) {",
                "  size_t len = strlen(key);",
                "  if (len != 16) return 0;",
                "  // the prefix every valid key starts with",
                "  return verify(key, \"PRISM-\");",
                "}",
            ]
            .iter()
            .enumerate()
            {
                super::code::line(ui, index, text, settings, &font);
            }
        });
}
