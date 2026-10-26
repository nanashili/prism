use super::theme::Palette;
use crate::model::ChatSummary;
use crate::ui::presentation::{ActivityState, Selection};
use eframe::egui::{self, Align2, FontId, Rect, Sense};

pub(super) fn chat_row(
    ui: &mut egui::Ui,
    chat: &ChatSummary,
    selected: Selection,
    busy: ActivityState,
) -> (egui::Response, egui::Response) {
    let selected = selected == Selection::Selected;
    let busy = busy == ActivityState::Active;

    let (rect, hover) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::hover());
    let menu_rect = Rect::from_min_max(egui::pos2(rect.right() - 28.0, rect.top()), rect.max);
    let open_rect = Rect::from_min_max(rect.min, egui::pos2(menu_rect.left(), rect.bottom()));
    let open = ui.interact(open_rect, ui.id().with((chat.id, "open")), Sense::click());
    let more = ui.interact(menu_rect, ui.id().with((chat.id, "menu")), Sense::click());
    let hovered = hover.hovered() || open.hovered() || more.hovered();
    paint_chat_row(ui, rect, menu_rect, chat, selected, busy, hovered);
    open.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            &chat.title,
        )
    });
    more.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!("Actions for {}", chat.title),
        )
    });
    (open.on_hover_text(&chat.title), more)
}

fn paint_chat_row(
    ui: &egui::Ui,
    rect: Rect,
    menu_rect: Rect,
    chat: &ChatSummary,
    selected: bool,
    busy: bool,
    hovered: bool,
) {
    let p = Palette::for_context(ui.ctx());
    if selected || hovered {
        ui.painter()
            .rect_filled(rect, 8.0, if selected { p.sel } else { p.fill });
    }
    let mut left = rect.left() + 10.0;
    if busy {
        ui.painter()
            .circle_filled(egui::pos2(left + 4.0, rect.center().y), 3.0, p.accent);
        left += 17.0;
    } else if chat.pinned {
        super::icons::Icon::Star.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(left + 6.0, rect.center().y),
                egui::vec2(12.0, 12.0),
            ),
            p.faint,
        );
        left += 20.0;
    }
    let when = if busy {
        "Working".into()
    } else {
        short_time(chat.updated)
    };
    let time = ui
        .painter()
        .layout_no_wrap(when, FontId::proportional(11.5), p.faint);
    let reserve = time.size().x.max(22.0) + 12.0;
    let mut title = egui::text::LayoutJob::simple_singleline(
        chat.title.clone(),
        FontId::proportional(13.0),
        if selected { p.text } else { p.text3 },
    );
    title.wrap.max_width = (rect.right() - left - reserve).max(0.0);
    title.wrap.max_rows = 1;
    let title = ui.fonts_mut(|fonts| fonts.layout_job(title));
    ui.painter().galley(
        egui::pos2(left, rect.center().y - title.size().y / 2.0),
        title,
        p.text3,
    );
    if hovered || selected {
        super::icons::Icon::More.paint(
            ui,
            Rect::from_center_size(menu_rect.center(), egui::vec2(15.0, 15.0)),
            p.muted,
        );
    } else {
        ui.painter().galley(
            rect.right_center() - egui::vec2(8.0 + time.size().x, time.size().y / 2.0),
            time,
            p.faint,
        );
    }
}

pub(super) fn project_row(
    ui: &mut egui::Ui,
    recent: &crate::model::Recent,
    selected: Selection,
) -> egui::Response {
    let selected = selected == Selection::Selected;

    let p = Palette::for_context(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, 8.0, p.sel);
    }
    let tile = Rect::from_min_size(rect.min + egui::vec2(8.0, 5.0), egui::vec2(34.0, 34.0));
    super::file_icons::paint(ui, &recent.original, tile);
    let clip = Rect::from_min_max(
        rect.min + egui::vec2(52.0, 0.0),
        rect.right_bottom() - egui::vec2(8.0, 0.0),
    );
    let painter = ui.painter().with_clip_rect(clip);
    painter.text(
        clip.min + egui::vec2(0.0, 14.0),
        Align2::LEFT_CENTER,
        &recent.title,
        FontId::proportional(13.5),
        p.text3,
    );
    let when = relative_time(recent.opened);
    let detail = match recent.chats {
        0 => when,
        1 => format!("1 chat · {when}"),
        count => format!("{count} chats · {when}"),
    };
    painter.text(
        clip.min + egui::vec2(0.0, 30.0),
        Align2::LEFT_CENTER,
        detail,
        FontId::proportional(11.5),
        p.faint.gamma_multiply(0.7),
    );
    response.on_hover_text(recent.path.display().to_string())
}

fn minutes_since(timestamp: i64) -> u64 {
    let timestamp = u64::try_from(timestamp).unwrap_or_default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(timestamp);
    now.saturating_sub(timestamp) / 60
}

fn short_time(timestamp: i64) -> String {
    let minutes = minutes_since(timestamp);
    match minutes {
        0 => "now".into(),
        1..60 => format!("{minutes}m"),
        60..1440 => format!("{}h", minutes / 60),
        1440..10080 => format!("{}d", minutes / 1440),
        _ => format!("{}w", minutes / 10080),
    }
}

pub(super) fn relative_time(timestamp: i64) -> String {
    let minutes = minutes_since(timestamp);
    match minutes {
        0 => "now".into(),
        1 => "1 minute ago".into(),
        2..60 => format!("{minutes} minutes ago"),
        60..120 => "1 hour ago".into(),
        120..1440 => format!("{} hours ago", minutes / 60),
        1440..2880 => "yesterday".into(),
        2880..10080 => format!("{} days ago", minutes / 1440),
        10080..20160 => "last week".into(),
        _ => format!("{} weeks ago", minutes / 10080),
    }
}
