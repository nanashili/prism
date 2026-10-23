use super::{app::Prism, icons::Icon, theme::Palette, widgets};
use crate::settings::research::Category;
use eframe::egui::{self, FontId, Rect, RichText, Stroke};

impl Prism {
    pub(super) fn workspace_categories(&mut self, ui: &mut egui::Ui, query: &str) {
        self.workspace_category_picker(ui, query);
        ui.add_space(16.0);
        ui.label(
            RichText::new("Your choices shape tool shortcuts, setup and skill suggestions. You can still open any supported file.")
                .size(12.0)
                .color(Palette::for_context(ui.ctx()).muted),
        );
    }

    pub(super) fn workspace_category_picker(&mut self, ui: &mut egui::Ui, query: &str) {
        self.workspace_category_actions(ui);
        ui.add_space(10.0);

        let columns = if ui.available_width() >= 680.0 {
            3
        } else if ui.available_width() >= 440.0 {
            2
        } else {
            1
        };
        let gap = 10.0;
        let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
        let visible = Category::ALL.into_iter().filter(|category| {
            let text = format!("{} {}", category.label(), category.description()).to_lowercase();
            query.is_empty() || text.contains(query) || "workspace categories".contains(query)
        });
        egui::Grid::new("workspace-categories")
            .num_columns(columns)
            .spacing([gap, gap])
            .show(ui, |ui| {
                for (index, category) in visible.enumerate() {
                    let selected = self.settings.research.contains(category);
                    let last = selected && self.settings.research.categories.len() == 1;
                    let response = category_tile(ui, category, selected, width);
                    if response.clicked() && !last {
                        self.settings.research.set(category, !selected);
                    }
                    if last {
                        response.on_hover_text("Keep at least one category selected.");
                    }
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    fn workspace_category_actions(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        ui.horizontal(|ui| {
            let count = self.settings.research.categories.len();
            ui.label(
                RichText::new(format!("{count} selected"))
                    .size(12.0)
                    .color(p.muted),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::chip(ui, None, "Select all", false, false).clicked() {
                    self.settings.research.categories = Category::ALL.to_vec();
                }
            });
        });
    }
}

fn category_tile(
    ui: &mut egui::Ui,
    category: Category,
    selected: bool,
    width: f32,
) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let response = ui.add_sized(
        [width, 104.0],
        egui::Button::new("")
            .fill(if selected { p.fill } else { p.card })
            .stroke(Stroke::new(1.0, if selected { p.line3 } else { p.line }))
            .corner_radius(12),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            12.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    let content = response.rect.shrink(14.0);
    let opacity = if response.hovered() { 0.72 } else { 1.0 };
    let title = ui.painter().layout(
        category.label().into(),
        FontId::proportional(13.5),
        p.text.gamma_multiply(opacity),
        content.width() - 24.0,
    );
    let detail = ui.painter().layout(
        category.description().into(),
        FontId::proportional(12.0),
        p.muted.gamma_multiply(opacity),
        content.width(),
    );
    let detail_top = content.top() + title.size().y + 7.0;
    let painter = ui.painter().with_clip_rect(content);
    painter.galley(content.min, title, p.text);
    painter.galley(egui::pos2(content.left(), detail_top), detail, p.muted);

    let check = Rect::from_min_size(
        content.right_top() - egui::vec2(15.0, 0.0),
        egui::vec2(15.0, 15.0),
    );
    if selected {
        Icon::Check.paint(ui, check, p.text.gamma_multiply(opacity));
    } else {
        painter.circle_stroke(check.center(), 6.0, Stroke::new(1.0, p.line3));
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            selected,
            category.label(),
        )
    });
    response
}
