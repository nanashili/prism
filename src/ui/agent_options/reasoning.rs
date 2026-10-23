use crate::agent::options::OptionMenu;
use crate::ui::{icons::Icon, theme::Palette};
use eframe::egui::{self, Align2, FontId, Rect, Sense};

mod slider;

pub(super) enum Action<'a> {
    Models,
    Pick(&'a OptionMenu, String),
}

pub(super) fn show<'a>(
    ui: &mut egui::Ui,
    effort: &'a OptionMenu,
    model: Option<&str>,
    fast: Option<&'a OptionMenu>,
) -> Option<Action<'a>> {
    let (header, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), Sense::hover());
    let (index, committed) = slider::show(ui, effort, model);
    let choice = &effort.choices[index];
    paint_title(ui, header, &choice.name);
    if model_link(ui, header, model) {
        return Some(Action::Models);
    }
    if let Some(value) = fast_button(ui, header, fast) {
        return fast.map(|menu| Action::Pick(menu, value));
    }
    if let Some(default) = effort
        .choices
        .iter()
        .find(|choice| choice.value == "medium")
        && header_button(
            ui,
            header.right_top() + egui::vec2(-24.0, 0.0),
            Icon::Retry,
            "Reset reasoning to Medium",
            false,
        )
        .clicked()
    {
        return Some(Action::Pick(effort, default.value.clone()));
    }
    committed.then(|| Action::Pick(effort, choice.value.clone()))
}

pub(super) fn model_header(ui: &mut egui::Ui, fast: Option<&OptionMenu>) -> Option<String> {
    let (header, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), Sense::hover());
    ui.painter().text(
        header.left_top() + egui::vec2(32.0, 12.0),
        Align2::LEFT_CENTER,
        "Fast mode",
        FontId::proportional(12.5),
        Palette::for_context(ui.ctx()).muted,
    );
    fast_button(ui, header, fast)
}

fn paint_title(ui: &egui::Ui, header: Rect, label: &str) {
    ui.painter().text(
        header.center_top() + egui::vec2(0.0, 12.0),
        Align2::CENTER_CENTER,
        effort_label(label),
        FontId::proportional(14.0),
        Palette::for_context(ui.ctx()).accent,
    );
}

fn model_link(ui: &mut egui::Ui, header: Rect, model: Option<&str>) -> bool {
    let bounds = Rect::from_center_size(
        header.center() + egui::vec2(0.0, 7.0),
        egui::vec2(header.width() - 64.0, 22.0),
    );
    let Some(model) = model else {
        return false;
    };
    let response = ui.interact(bounds, ui.id().with("select-model"), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!("Select model: {model}"),
        )
    });
    let color = Palette::for_context(ui.ctx()).muted;
    let mut job = egui::text::LayoutJob::simple_singleline(
        format!("{model} ›"),
        FontId::proportional(11.5),
        color,
    );
    job.wrap.max_rows = 1;
    job.wrap.max_width = bounds.width();
    let text = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter()
        .galley(bounds.center() - text.size() / 2.0, text, color);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn fast_button(ui: &mut egui::Ui, header: Rect, menu: Option<&OptionMenu>) -> Option<String> {
    let enabled = menu.is_some_and(|menu| menu.current == "on");
    let response = ui
        .add_enabled_ui(menu.is_some(), |ui| {
            header_button(ui, header.left_top(), Icon::Bolt, "Fast mode", enabled)
        })
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled() && menu.is_some(),
            enabled,
            "Fast mode",
        )
    });
    let description = menu
        .and_then(|menu| menu.choices.iter().find(|choice| choice.value == "on"))
        .map_or("This agent or model does not offer Fast mode.", |choice| {
            choice.description.as_str()
        });
    let clicked = response
        .on_disabled_hover_text(description)
        .on_hover_text(format!(
            "Fast mode: {}\n{description}",
            if enabled { "On" } else { "Off" }
        ))
        .clicked();
    clicked.then(|| if enabled { "off".into() } else { "on".into() })
}

fn header_button(
    ui: &mut egui::Ui,
    position: egui::Pos2,
    icon: Icon,
    label: &str,
    selected: bool,
) -> egui::Response {
    let rect = Rect::from_min_size(position, egui::vec2(24.0, 24.0));
    let response = ui.interact(rect, ui.id().with(label), Sense::click());
    let p = Palette::for_context(ui.ctx());
    if response.hovered() || response.has_focus() || selected {
        ui.painter().rect_filled(rect, 6.0, p.fill2);
    }
    icon.paint(
        ui,
        rect.shrink(5.0),
        if selected {
            p.accent
        } else if ui.is_enabled() {
            p.muted
        } else {
            p.faint
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response.on_hover_text(label)
}

fn effort_label(name: &str) -> &str {
    match name {
        "Xhigh" | "XHigh" | "xhigh" => "Extra High",
        _ => name,
    }
}
