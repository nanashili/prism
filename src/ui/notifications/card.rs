use super::state::{Level, Notice};
use crate::ui::{icons::Icon, theme::Palette, widgets};
use eframe::egui::{self, Color32, Frame, RichText, Stroke};
use std::time::{Duration, Instant};

pub(super) fn show(
    ui: &mut egui::Ui,
    entry: &mut Notice,
    can_open_chat: bool,
    history: bool,
) -> bool {
    let p = Palette::for_context(ui.ctx());
    let color = tone(entry.level, ui.visuals().dark_mode);
    let card = Frame::NONE
        .fill(p.raised)
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.45)))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| contents(ui, entry, can_open_chat, history));
    let response = card.response;
    if !history {
        let focused = ui.memory(|memory| memory.focused()).is_some_and(|id| {
            ui.ctx()
                .read_response(id)
                .is_some_and(|r| response.rect.intersects(r.rect))
        });
        if response.contains_pointer() || focused || entry.expanded {
            entry.keep_visible();
        }
        if let Some(remaining) = entry.remaining() {
            ui.ctx().request_repaint_after(remaining);
        }
    }
    card.inner
}

fn contents(ui: &mut egui::Ui, entry: &mut Notice, can_open_chat: bool, history: bool) -> bool {
    let p = Palette::for_context(ui.ctx());
    header(
        ui,
        entry,
        tone(entry.level, ui.visuals().dark_mode),
        history,
    );
    if !entry.scope.is_empty() {
        ui.add(egui::Label::new(RichText::new(&entry.scope).size(11.0).color(p.muted)).truncate());
    }
    if !entry.body.is_empty() {
        ui.add_space(2.0);
        preview(ui, &entry.body, p.text3);
    }
    let open_chat = if !entry.body.is_empty() || !entry.details.is_empty() || can_open_chat {
        ui.add_space(7.0);
        ui.horizontal_wrapped(|ui| actions(ui, entry, can_open_chat))
            .inner
    } else {
        false
    };
    if entry.expanded {
        details(ui, entry, history);
    }
    open_chat
}

fn details(ui: &mut egui::Ui, entry: &Notice, history: bool) {
    let p = Palette::for_context(ui.ctx());
    ui.add_space(6.0);
    Frame::NONE
        .fill(p.deep)
        .corner_radius(7)
        .inner_margin(8)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt((entry.id, "details", history))
                .max_height(112.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&entry.details).monospace().size(11.5))
                            .wrap()
                            .selectable(true),
                    );
                });
        });
}

fn header(ui: &mut egui::Ui, entry: &mut Notice, color: Color32, history: bool) {
    ui.horizontal(|ui| {
        let icon = match entry.level {
            Level::Success => Icon::Check,
            Level::Error | Level::Warning => Icon::ShieldAlert,
            Level::Info => Icon::Info,
        };
        icon.show(ui, 15.0, color);
        let width = (ui.available_width() - 25.0).max(30.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, 20.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_size(egui::vec2(width, 20.0));
                ui.add(egui::Label::new(RichText::new(&entry.title).size(13.0).strong()).truncate())
            },
        );
        if !history && widgets::icon_button(ui, Icon::Close, "Dismiss notification", 20.0).clicked()
        {
            entry.dismiss();
        }
    });
    if history || entry.occurrences > 1 {
        let repeated = if entry.occurrences > 1 {
            format!(" · {} times", entry.occurrences)
        } else {
            String::new()
        };
        let minutes = entry.updated.elapsed().as_secs() / 60;
        let age = if minutes == 0 {
            "Just now".into()
        } else {
            format!("{minutes}m ago")
        };
        ui.label(RichText::new(format!("{age}{repeated}")).size(11.0).weak());
    }
}

fn preview(ui: &mut egui::Ui, text: &str, color: Color32) {
    let mut job = egui::text::LayoutJob::simple(
        text.into(),
        egui::FontId::proportional(12.5),
        color,
        ui.available_width(),
    );
    job.wrap.max_rows = 3;
    job.wrap.break_anywhere = true;
    ui.add(egui::Label::new(job).selectable(true));
}

fn actions(ui: &mut egui::Ui, entry: &mut Notice, can_open_chat: bool) -> bool {
    let copied = entry
        .copied
        .is_some_and(|time| time.elapsed() < Duration::from_secs(2));
    if ui
        .small_button(if copied { "Copied" } else { "Copy details" })
        .clicked()
    {
        ui.ctx().copy_text(entry.copy_text());
        entry.copied = Some(Instant::now());
        ui.ctx().request_repaint_after(Duration::from_secs(2));
    }
    if !entry.details.is_empty()
        && ui
            .small_button(if entry.expanded {
                "Hide details"
            } else {
                "Show details"
            })
            .clicked()
    {
        entry.expanded = !entry.expanded;
    }
    can_open_chat && ui.small_button("Open chat").clicked()
}

fn tone(level: Level, dark: bool) -> Color32 {
    match (level, dark) {
        (Level::Error, true) => Color32::from_rgb(224, 108, 117),
        (Level::Error, false) => Color32::from_rgb(180, 45, 60),
        (Level::Warning, true) => Color32::from_rgb(229, 192, 123),
        (Level::Warning, false) => Color32::from_rgb(147, 96, 14),
        (Level::Success, true) => Color32::from_rgb(152, 195, 121),
        (Level::Success, false) => Color32::from_rgb(54, 121, 63),
        (Level::Info, _) => {
            if dark {
                Color32::from_rgb(115, 173, 233)
            } else {
                Color32::from_rgb(47, 111, 208)
            }
        }
    }
}
