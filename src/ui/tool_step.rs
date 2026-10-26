use super::message::{
    self,
    style::{BlockKind, ChatStyle},
};
use super::{activity_view::duration, icons::Icon, theme::Palette};
use crate::model::{Outcome, ToolStep};
use crate::ui::presentation::Disclosure;
use eframe::egui::{self, Align2, FontId, Rect, RichText, Sense};

pub(super) fn show(ui: &mut egui::Ui, tool: &ToolStep, style: &ChatStyle) {
    let suffix = match tool.status {
        Outcome::Running => "Working…".into(),
        Outcome::Failed => "Didn't work".into(),
        Outcome::Stopped => "Stopped".into(),
        Outcome::Done => match tool.ended {
            Some(end) if end - tool.started < 10_000 => {
                format!("{} ms", (end - tool.started).max(1))
            }
            Some(end) => duration(end - tool.started),
            None => "Done".into(),
        },
    };
    let label = if tool.title.is_empty() {
        "Using a tool".into()
    } else {
        tool.title.replace('_', " ")
    };
    let id = ui.id().with(&tool.id);
    if disclosure(ui, id, Icon::Bolt, &label, &suffix, Disclosure::Closed) {
        style.block(ui, BlockKind::Tool, |ui| {
            for (name, text) in [("Input", &tool.input), ("Output", &tool.output)] {
                if text.is_empty() {
                    continue;
                }
                ui.add_space(6.0);
                ui.label(RichText::new(name).small().weak());
                egui::ScrollArea::vertical()
                    .id_salt(name)
                    .max_height(220.0)
                    .show(ui, |ui| {
                        message::code_text(ui, text, style);
                    });
            }
            if tool.input.is_empty() && tool.output.is_empty() {
                ui.label(
                    RichText::new("This agent did not provide tool details.")
                        .small()
                        .weak(),
                );
            }
        });
    }
}

pub(super) fn disclosure(
    ui: &mut egui::Ui,
    id: egui::Id,
    icon: Icon,
    label: &str,
    suffix: &str,
    default_open: Disclosure,
) -> bool {
    let default_open = default_open == Disclosure::Open;

    let mut open = ui
        .data(|data| data.get_temp::<bool>(id))
        .unwrap_or(default_open);
    let p = Palette::for_context(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(
            ui.available_width(),
            ui.text_style_height(&egui::TextStyle::Body).max(20.0) + 10.0,
        ),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter().rect_filled(rect, 5.0, p.well);
    }
    paint_disclosure(ui, rect, icon, label, suffix, open);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, ui.is_enabled(), label)
    });
    let response = response.on_hover_text(label);
    if response.clicked() {
        open = !open;
        ui.data_mut(|data| data.insert_temp(id, open));
    }
    open
}

fn paint_disclosure(ui: &egui::Ui, rect: Rect, icon: Icon, label: &str, suffix: &str, open: bool) {
    let p = Palette::for_context(ui.ctx());
    let icon_rect = Rect::from_center_size(
        rect.left_center() + egui::vec2(9.0, 0.0),
        egui::vec2(16.0, 16.0),
    );
    icon.paint(ui, icon_rect, p.muted);
    let tool = !suffix.is_empty();
    let left = rect.left() + if tool { 66.0 } else { 28.0 };
    if tool {
        paint_tool_badge(ui, rect);
    }
    let trailing = ui.painter().layout_no_wrap(
        suffix.into(),
        FontId::proportional(egui::TextStyle::Body.resolve(ui.style()).size - 1.0),
        p.faint,
    );
    let suffix_left = (rect.right() - 25.0 - trailing.size().x).max(left);
    let title_right = if tool {
        suffix_left - 8.0
    } else {
        rect.right() - 24.0
    };
    let title_rect = Rect::from_min_max(
        egui::pos2(left, rect.top()),
        egui::pos2(title_right.max(left), rect.bottom()),
    );
    paint_title(ui, title_rect, label);
    let suffix_rect = Rect::from_min_max(
        egui::pos2(suffix_left, rect.top()),
        egui::pos2((rect.right() - 25.0).max(suffix_left), rect.bottom()),
    );
    ui.painter()
        .with_clip_rect(suffix_rect.intersect(ui.clip_rect()))
        .galley(
            egui::pos2(suffix_left, rect.center().y - trailing.size().y / 2.0),
            trailing,
            p.faint,
        );
    let caret = if open {
        Icon::ChevronUp
    } else {
        Icon::Disclosure
    };
    caret.paint(
        ui,
        Rect::from_center_size(
            rect.right_center() - egui::vec2(8.0, 0.0),
            egui::vec2(12.0, 12.0),
        ),
        p.faint,
    );
}

fn paint_tool_badge(ui: &egui::Ui, row: Rect) {
    let p = Palette::for_context(ui.ctx());
    let badge = Rect::from_min_size(row.min + egui::vec2(26.0, 4.0), egui::vec2(32.0, 20.0));
    ui.painter().rect_filled(badge, 5.0, p.fill2);
    ui.painter().text(
        badge.center(),
        Align2::CENTER_CENTER,
        "Tool",
        FontId::proportional(11.0),
        p.muted,
    );
}

fn paint_title(ui: &egui::Ui, rect: Rect, label: &str) {
    if rect.width() <= 0.0 {
        return;
    }
    let color = Palette::for_context(ui.ctx()).text3;
    let mut job = egui::text::LayoutJob::simple_singleline(
        label.into(),
        egui::TextStyle::Body.resolve(ui.style()),
        color,
    );
    job.wrap.max_rows = 1;
    job.wrap.max_width = rect.width();
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let title = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(
            egui::pos2(rect.left(), rect.center().y - title.size().y / 2.0),
            title,
            color,
        );
}
