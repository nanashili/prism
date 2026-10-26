use super::{Command, Entry, Palette};
use crate::ui::presentation::InstallationState;
use crate::ui::{agent_logos, icons::Icon};
use eframe::egui::{self, FontId, Rect, Stroke};

pub(super) fn grid(
    ui: &mut egui::Ui,
    entries: &[&Entry],
    selected: &str,
    installed: InstallationState,
) -> Option<Command> {
    let installed = installed == InstallationState::Installed;

    let columns = if ui.available_width() >= 520.0 { 2 } else { 1 };
    let gap = 10.0;
    let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
    let mut picked = None;
    egui::Grid::new(("setup-agent-cards", installed))
        .num_columns(columns)
        .spacing([gap, gap])
        .show(ui, |ui| {
            for (index, entry) in entries.iter().enumerate() {
                if card(ui, entry, entry.id == selected, installed, width).clicked() {
                    picked = Some(Command::Agent(entry.id.clone(), entry.name.clone()));
                }
                if (index + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
    picked
}

fn card(
    ui: &mut egui::Ui,
    entry: &Entry,
    selected: bool,
    installed: bool,
    width: f32,
) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let response = ui
        .push_id(&entry.id, |ui| {
            ui.add_sized(
                [width, 80.0],
                egui::Button::new("")
                    .fill(if selected { p.fill } else { p.well })
                    .stroke(Stroke::new(1.0, if selected { p.line3 } else { p.line }))
                    .corner_radius(12),
            )
        })
        .inner;
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            12.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }

    let content = response.rect.shrink(16.0);
    let logo = Rect::from_center_size(
        egui::pos2(content.left() + 18.0, content.center().y),
        egui::vec2(36.0, 36.0),
    );
    agent_logos::paint(ui, &entry.id, logo);
    let text = Rect::from_min_max(
        egui::pos2(logo.right() + 14.0, content.top()),
        content.right_bottom() - egui::vec2(26.0, 0.0),
    );
    card_text(ui, entry, text, selected, installed, response.hovered());

    let check = Rect::from_center_size(
        egui::pos2(content.right() - 8.0, content.center().y),
        egui::vec2(16.0, 16.0),
    );
    if selected {
        Icon::Check.paint(ui, check, p.text);
    } else {
        ui.painter()
            .circle_stroke(check.center(), 6.0, Stroke::new(1.0, p.line3));
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            ui.is_enabled(),
            selected,
            format!(
                "{}, {}",
                entry.name,
                if installed {
                    "installed"
                } else {
                    "not installed"
                }
            ),
        )
    });
    response
}

fn card_text(
    ui: &egui::Ui,
    entry: &Entry,
    rect: Rect,
    selected: bool,
    installed: bool,
    hovered: bool,
) {
    let p = Palette::for_context(ui.ctx());
    let opacity = if hovered { 0.72 } else { 1.0 };
    let title = ui.painter().layout_no_wrap(
        entry.name.clone(),
        FontId::proportional(14.0),
        p.text.gamma_multiply(opacity),
    );
    let status = match (installed, selected) {
        (true, true) => "Installed · Selected",
        (true, false) => "Installed",
        (false, true) => "Selected · Not installed",
        (false, false) => "Not installed",
    };
    let detail = ui.painter().layout_no_wrap(
        status.into(),
        FontId::proportional(11.5),
        p.muted.gamma_multiply(opacity),
    );
    let height = title.size().y + 5.0 + detail.size().y;
    let position = egui::pos2(rect.left(), rect.center().y - height * 0.5);
    let detail_position = position + egui::vec2(0.0, title.size().y + 5.0);
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.galley(position, title, p.text);
    painter.galley(detail_position, detail, p.muted);
}
