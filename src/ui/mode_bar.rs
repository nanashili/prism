use super::app::Prism;
use super::{function_bar::view_icon, icons::Icon, theme::Palette, widgets};
use crate::model::View;
use eframe::egui::{self, Align2, FontId, Frame, Rect, Sense, Stroke};

const MODES: [(View, &str); 4] = [
    (View::Assembly, "Listing"),
    (View::Pseudocode, "Pseudocode"),
    (View::Graph, "Graph"),
    (View::Hex, "Hex"),
];

impl Prism {
    pub(super) fn function_mode_bar(&mut self, ui: &mut egui::Ui) {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::hover());
        let words = rect.width() >= 640.0;
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(egui::vec2(10.0, 3.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let modes_width = if words {
            MODES
                .iter()
                .map(|(_, label)| {
                    ui.painter()
                        .layout_no_wrap(
                            (*label).into(),
                            FontId::proportional(12.0),
                            Palette::for_context(ui.ctx()).text,
                        )
                        .size()
                        .x
                        + 38.0
                })
                .sum::<f32>()
                + 10.0
        } else {
            122.0
        };
        let address_width = (row.available_width() - modes_width - 42.0).clamp(60.0, 260.0);
        self.address_field(&mut row, address_width);
        row.add_space((row.available_width() - modes_width - 34.0).max(0.0));
        self.mode_switch(&mut row, words);
        if widgets::icon_button(&mut row, Icon::Bookmark, "Bookmark", 26.0).clicked() {
            let args = serde_json::json!({"address": self.workspace.selected});
            if let Err(error) = self.apply_annotation_tool("bookmark", &args) {
                self.error = Some(error.to_string());
            }
        }
    }

    fn address_field(&mut self, ui: &mut egui::Ui, width: f32) {
        let p = Palette::for_context(ui.ctx());
        Frame::NONE
            .fill(p.deep)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(7)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.set_width(width - 16.0);
                let hint = format!("G   {}", self.workspace.selected);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.workspace.jump)
                        .frame(false)
                        .font(FontId::monospace(12.0))
                        .hint_text(hint)
                        .desired_width(width - 16.0),
                );
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    self.go_to_function();
                }
            });
    }

    fn mode_switch(&mut self, ui: &mut egui::Ui, words: bool) {
        let p = Palette::for_context(ui.ctx());
        Frame::NONE
            .fill(p.fill)
            .corner_radius(8)
            .inner_margin(2)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (view, label) in MODES {
                        if mode_button(ui, view, label, self.workspace.view == view, words)
                            .clicked()
                        {
                            self.select_view(view);
                        }
                    }
                });
            });
    }

    fn go_to_function(&mut self) {
        let query = self.workspace.jump.trim();
        let destination = self
            .workspace
            .rows
            .iter()
            .find(|row| row.name == query)
            .map(|row| row.address.clone())
            .or_else(|| {
                u64::from_str_radix(query.trim_start_matches("0x"), 16)
                    .ok()
                    .map(|address| format!("0x{address:x}"))
            });
        if let Some(address) = destination {
            self.workspace.selected = address;
            self.workspace.jump.clear();
            self.select_view(self.workspace.view);
        } else {
            self.error = Some("Enter a function name or hexadecimal address".into());
        }
    }
}

fn mode_button(
    ui: &mut egui::Ui,
    view: View,
    label: &str,
    selected: bool,
    words: bool,
) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let width = if words {
        ui.painter()
            .layout_no_wrap(label.into(), FontId::proportional(12.0), p.text)
            .size()
            .x
            + 38.0
    } else {
        28.0
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 6.0, p.sel);
    }
    view_icon(view).paint(
        ui,
        Rect::from_min_size(rect.min + egui::vec2(7.0, 5.0), egui::vec2(14.0, 14.0)),
        p.muted,
    );
    if words {
        ui.painter().text(
            rect.min + egui::vec2(27.0, 12.0),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.0),
            p.text3,
        );
    }
    response.on_hover_text(label)
}
