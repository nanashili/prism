use super::{key_combo, keyboard_edit::Editor, popup, shortcut_catalog, theme::Palette};
use crate::settings::Settings;
use crate::ui::presentation::Selection;
use eframe::egui::{self, Align2, FontId, RichText};

const KEYMAPS: &[&str] = &[
    "Prism",
    "Hopper",
    "Ghidra",
    "Cursor or VS Code",
    "Zed",
    "Xcode",
];

pub(super) fn header(ui: &mut egui::Ui, settings: &mut Settings, editor: &mut Editor) {
    let p = Palette::for_context(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.label(RichText::new("Based on").size(12.5).color(p.muted));
        keymap(ui, &mut settings.views.keymap);
        let changed = settings.views.shortcuts.len();
        if changed > 0 {
            ui.label(
                RichText::new(format!(
                    "{changed} {}",
                    if changed == 1 { "change" } else { "changes" }
                ))
                .size(12.0)
                .color(p.muted),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let response = ui.add_sized(
                [84.0, 30.0],
                egui::Button::new("↓  Import")
                    .corner_radius(8)
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke((1.0, p.line3)),
            );
            popup::menu(&response, 280.0)
                .align(egui::RectAlign::BOTTOM_END)
                .show(|ui| {
                    popup::heading(ui, "Import shortcuts");
                    if popup::row_with_height(
                        ui,
                        None,
                        "Choose a file",
                        "",
                        Selection::Unselected,
                        36.0,
                    )
                    .clicked()
                    {
                        editor.import(settings);
                        ui.close();
                    }
                    ui.label(
                        RichText::new("Prism JSON command-to-shortcut map")
                            .size(11.5)
                            .color(p.muted),
                    );
                });
            if changed > 0
                && ui
                    .add_sized(
                        [80.0, 30.0],
                        egui::Button::new(RichText::new("Reset all").color(p.muted)).frame(false),
                    )
                    .clicked()
            {
                settings.views.shortcuts.clear();
                editor.conflict = None;
            }
        });
    });
}

fn keymap(ui: &mut egui::Ui, value: &mut String) {
    let p = Palette::for_context(ui.ctx());
    let text = ui
        .painter()
        .layout_no_wrap(value.clone(), FontId::proportional(12.5), p.text);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(text.size().x + 68.0, 30.0), egui::Sense::click());
    ui.painter()
        .rect_stroke(rect, 8, (1.0, p.line3), egui::StrokeKind::Inside);
    keymap_icon(
        ui,
        value,
        egui::Rect::from_center_size(
            rect.left_center() + egui::vec2(18.0, 0.0),
            egui::vec2(18.0, 18.0),
        ),
    );
    ui.painter().galley(
        rect.left_center() + egui::vec2(35.0, -text.size().y * 0.5),
        text,
        p.text,
    );
    super::icons::Icon::UpDown.paint(
        ui,
        egui::Rect::from_center_size(
            rect.right_center() - egui::vec2(13.0, 0.0),
            egui::vec2(11.0, 11.0),
        ),
        p.muted,
    );
    popup::menu(&response, 220.0)
        .align(egui::RectAlign::BOTTOM_END)
        .show(|ui| {
            for &option in KEYMAPS {
                let response = popup::row_with_height(
                    ui,
                    None,
                    "",
                    if value == option { "✓" } else { "" },
                    Selection::from(value == option),
                    34.0,
                );
                let center = response.rect.left_center();
                keymap_icon(
                    ui,
                    option,
                    egui::Rect::from_center_size(
                        center + egui::vec2(17.0, 0.0),
                        egui::vec2(18.0, 18.0),
                    ),
                );
                ui.painter().text(
                    center + egui::vec2(34.0, 0.0),
                    Align2::LEFT_CENTER,
                    option,
                    FontId::proportional(12.5),
                    p.text,
                );
                if response.clicked() {
                    *value = option.into();
                    ui.close();
                }
            }
        });
}

fn keymap_icon(ui: &egui::Ui, keymap: &str, rect: egui::Rect) {
    let image = match keymap {
        "Prism" => Some(crate::ui::branding::LOGO),
        "Hopper" => Some(egui::include_image!(
            "../../resources/upstream/engines/hopper.png"
        )),
        "Ghidra" => Some(egui::include_image!(
            "../../resources/upstream/engines/ghidra.png"
        )),
        _ => None,
    };
    if let Some(image) = image {
        egui::Image::new(image).paint_at(ui, rect);
        return;
    }
    let (app, letters) = match keymap {
        "Cursor or VS Code" => ("Visual Studio Code", "VS"),
        "Zed" => ("Zed", "Z"),
        _ => ("Xcode", "X"),
    };
    let path = std::path::PathBuf::from(format!("/Applications/{app}.app"));
    if path.exists() {
        super::file_icons::paint(ui, &path, rect);
    } else {
        let p = Palette::for_context(ui.ctx());
        ui.painter().rect_filled(rect, 4, p.fill2);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            letters,
            FontId::proportional(8.0),
            p.text3,
        );
    }
}

pub(super) fn conflict(ui: &mut egui::Ui, settings: &mut Settings, editor: &mut Editor, id: &str) {
    let Some((target, combo, other)) = editor.conflict.clone() else {
        return;
    };
    if target != id {
        return;
    }
    let label = shortcut_catalog::COMMANDS
        .iter()
        .find(|(id, ..)| *id == other)
        .map_or(other.as_str(), |(_, label, ..)| *label);
    let p = Palette::for_context(ui.ctx());
    egui::Frame::NONE
        .outer_margin(egui::Margin {
            left: 18,
            right: 10,
            top: 0,
            bottom: 10,
        })
        .inner_margin(egui::Margin::symmetric(10, 8))
        .corner_radius(10)
        .fill(egui::Color32::from_rgba_unmultiplied(229, 192, 123, 24))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "{} is already {label}. Use it here and leave {label} without a shortcut?",
                    key_combo::text(&combo)
                ))
                .size(12.5)
                .color(p.text3),
            );
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    editor.conflict = None;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("Use here").color(p.card))
                            .fill(p.text)
                            .corner_radius(7),
                    )
                    .clicked()
                {
                    editor.accept_conflict(settings);
                }
            });
        });
}
