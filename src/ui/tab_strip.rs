use super::{icons::Icon, theme::Palette, widgets};
use eframe::egui::{self, Color32, FontId, Rect, Response, Sense};

pub(super) const HEIGHT: f32 = 36.0;

pub(super) struct Tab<'a> {
    pub title: &'a str,
    pub active: bool,
    pub focused: bool,
    pub preview: bool,
    pub busy: bool,
}

pub(super) struct TabResponse {
    pub response: Response,
    pub close_requested: bool,
}

pub(super) fn show(
    ui: &mut egui::Ui,
    controls_width: f32,
    contents: impl FnOnce(&mut egui::Ui),
) -> Rect {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), HEIGHT), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, Palette::for_context(ui.ctx()).deep);

    let divider = (rect.right() - controls_width).max(rect.left());
    let tabs_rect = Rect::from_min_max(rect.min, egui::pos2(divider, rect.bottom()));
    let mut tabs = ui.new_child(egui::UiBuilder::new().max_rect(tabs_rect));
    tabs.set_clip_rect(tabs_rect.intersect(ui.clip_rect()));
    egui::ScrollArea::horizontal()
        .id_salt("tab-strip")
        .max_height(HEIGHT)
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut tabs, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                contents(ui);
            });
        });
    Rect::from_min_max(egui::pos2(divider, rect.top()), rect.max)
}

pub(super) fn tab(
    ui: &mut egui::Ui,
    tab: Tab<'_>,
    paint_icon: impl FnOnce(&egui::Ui, Rect, Color32),
) -> TabResponse {
    let p = Palette::for_context(ui.ctx());
    let text = label(ui, &tab);
    let busy_width = if tab.busy { 10.0 } else { 0.0 };
    let width = text.size().x.min(180.0) + 61.0 + busy_width;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, HEIGHT), Sense::click());
    paint_background(ui, rect, &tab);
    paint_icon(
        ui,
        Rect::from_min_size(rect.min + egui::vec2(12.0, 11.0), egui::vec2(14.0, 14.0)),
        if tab.active { p.accent } else { p.muted },
    );
    let label_rect = Rect::from_min_max(
        rect.min + egui::vec2(33.0, 0.0),
        rect.right_bottom() - egui::vec2(28.0 + busy_width, 0.0),
    );
    ui.painter()
        .with_clip_rect(label_rect.intersect(ui.clip_rect()))
        .galley(
            label_rect.left_center() - egui::vec2(0.0, text.size().y * 0.5),
            text,
            p.text,
        );

    let close = Rect::from_min_size(
        rect.right_top() + egui::vec2(-28.0, 7.0),
        egui::vec2(22.0, 22.0),
    );
    if tab.busy {
        ui.painter()
            .circle_filled(close.left_center() - egui::vec2(5.0, 0.0), 3.0, p.accent);
    }
    let close_requested =
        close_button(ui, close, &response) || response.clicked_by(egui::PointerButton::Middle);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            tab.active,
            tab.title,
        )
    });
    TabResponse {
        response: response.on_hover_text(tab.title),
        close_requested,
    }
}

fn label(ui: &egui::Ui, tab: &Tab<'_>) -> std::sync::Arc<egui::Galley> {
    let p = Palette::for_context(ui.ctx());
    let format = egui::TextFormat {
        font_id: FontId::proportional(13.0),
        color: if tab.active { p.text } else { p.muted },
        italics: tab.preview,
        ..Default::default()
    };
    ui.fonts_mut(|fonts| {
        fonts.layout_job(egui::text::LayoutJob::single_section(
            tab.title.into(),
            format,
        ))
    })
}

fn paint_background(ui: &egui::Ui, rect: Rect, tab: &Tab<'_>) {
    if !tab.active {
        return;
    }
    let p = Palette::for_context(ui.ctx());
    ui.painter()
        .rect_filled(rect, 0.0, if tab.focused { p.card } else { p.deep });
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0)),
        0.0,
        if tab.focused { p.accent } else { p.line3 },
    );
}

fn close_button(ui: &mut egui::Ui, rect: Rect, tab: &Response) -> bool {
    if !tab.hovered() && !ui.rect_contains_pointer(rect) {
        return false;
    }
    let mut close_ui = ui.new_child(egui::UiBuilder::new().id_salt("close-tab").max_rect(rect));
    widgets::icon_button(&mut close_ui, Icon::Close, "Close tab", 22.0).clicked()
}
