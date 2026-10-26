use super::{key_combo, keyboard_edit::Editor, shortcut_catalog as catalog, theme::Palette};
use crate::settings::Settings;
use crate::ui::presentation::RowPosition;
use eframe::egui::{self, RichText};

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings, editor: &mut Editor, query: &str) {
    super::keyboard_controls::header(ui, settings, editor);
    ui.add_space(14.0);
    if query.is_empty() {
        super::keyboard_map::show(ui, settings, editor);
    } else {
        shortcut_list(ui, settings, editor, query);
    }
}

fn shortcut_list(ui: &mut egui::Ui, settings: &mut Settings, editor: &mut Editor, query: &str) {
    let p = Palette::for_context(ui.ctx());
    egui::Frame::NONE
        .fill(p.fill2)
        .corner_radius(9)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                super::icons::Icon::Search.show(ui, 14.0, p.faint);
                ui.add(
                    egui::TextEdit::singleline(&mut editor.query)
                        .frame(false)
                        .desired_width(ui.available_width())
                        .hint_text("Search by name or keys"),
                );
            });
        });
    ui.add_space(22.0);
    if let Some(error) = &editor.import_error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
    let needle = if query.is_empty() {
        editor.query.to_lowercase()
    } else {
        query.to_lowercase()
    };
    for &group in catalog::GROUPS {
        let rows: Vec<_> = catalog::COMMANDS
            .iter()
            .filter(|(id, label, section, default)| {
                *section == group
                    && format!(
                        "{group} {label} Keyboard {}",
                        key_combo::text(catalog::combo(settings, id, default))
                    )
                    .to_lowercase()
                    .contains(&needle)
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        ui.label(RichText::new(group).size(12.5).color(p.muted));
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(p.raised)
            .corner_radius(14)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for (index, &&(id, label, _, default)) in rows.iter().enumerate() {
                    row(
                        ui,
                        settings,
                        editor,
                        (id, label, default),
                        RowPosition::from(index == 0),
                    );
                }
            });
        ui.add_space(26.0);
    }
}

pub(super) fn row(
    ui: &mut egui::Ui,
    settings: &mut Settings,
    editor: &mut Editor,
    command: (&str, &str, &str),
    first: RowPosition,
) {
    let first = first == RowPosition::First;

    let (id, label, default) = command;
    let p = Palette::for_context(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), egui::Sense::hover());
    if !first {
        ui.painter().hline(
            rect.left() + 18.0..=rect.right() - 18.0,
            rect.top(),
            (1.0, p.line),
        );
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(18.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.5),
        p.text,
    );
    let recording = editor.recording.as_deref() == Some(id);
    let combo = catalog::combo(settings, id, default);
    let bounds = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 62.0, rect.center().y),
        egui::vec2(104.0, 30.0),
    );
    let response = ui.interact(bounds, ui.id().with((id, "shortcut")), egui::Sense::click());
    if recording {
        ui.painter()
            .rect_stroke(bounds, 8, (1.0, p.accent), egui::StrokeKind::Inside);
        ui.painter().text(
            bounds.center(),
            egui::Align2::CENTER_CENTER,
            "Press keys…",
            egui::FontId::proportional(12.0),
            p.accent,
        );
    } else {
        paint_keys(ui, bounds, combo);
    }
    if catalog::supported(id) {
        if response.clicked() {
            editor.recording = if recording { None } else { Some(id.into()) };
            ui.memory_mut(|memory| memory.surrender_focus(response.id));
            editor.conflict = None;
        }
    } else {
        response.on_hover_text("This command is not available in the Rust workbench yet.");
    }
    if settings.views.shortcuts.contains_key(id) {
        let reset = egui::Rect::from_center_size(
            egui::pos2(bounds.left() - 32.0, bounds.center().y),
            egui::vec2(48.0, 24.0),
        );
        if ui
            .put(
                reset,
                egui::Button::new(RichText::new("Reset").size(11.0)).frame(false),
            )
            .clicked()
        {
            settings.views.shortcuts.remove(id);
        }
    }
    super::keyboard_controls::conflict(ui, settings, editor, id);
}

fn paint_keys(ui: &egui::Ui, bounds: egui::Rect, combo: &str) {
    let p = Palette::for_context(ui.ctx());
    let caps = key_combo::caps(combo);
    if caps.is_empty() {
        ui.painter().text(
            bounds.right_center() - egui::vec2(6.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "Add",
            egui::FontId::proportional(12.0),
            p.faint,
        );
        return;
    }
    let mut right = bounds.right() - 6.0;
    for cap in caps.iter().rev() {
        let text =
            ui.painter()
                .layout_no_wrap(cap.clone(), egui::FontId::proportional(11.0), p.muted);
        let width = (text.size().x + 8.0).max(18.0);
        let rect = egui::Rect::from_min_size(
            egui::pos2(right - width, bounds.center().y - 9.0),
            egui::vec2(width, 18.0),
        );
        ui.painter().rect_filled(rect, 4, p.fill2);
        ui.painter()
            .galley(rect.center() - text.size() * 0.5, text, p.muted);
        right -= width + 3.0;
    }
}
