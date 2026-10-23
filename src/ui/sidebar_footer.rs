use super::app::Prism;
use super::{icons::Icon, popup, theme::Palette, widgets};
use crate::model::Screen;
use eframe::egui::{self, Align2, FontId, Rect, Sense};

impl Prism {
    pub(super) fn sidebar_footer(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let p = Palette::for_context(ui.ctx());
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width() - 72.0, 34.0),
                Sense::click(),
            );
            if response.hovered() {
                ui.painter().rect_filled(rect, 8.0, p.sel);
            }
            let logo = Rect::from_center_size(
                egui::pos2(rect.left() + 18.0, rect.center().y),
                egui::vec2(20.0, 20.0),
            );
            egui::Image::new(super::branding::LOGO).paint_at(ui, logo);
            ui.painter().text(
                egui::pos2(rect.left() + 37.0, rect.center().y),
                Align2::LEFT_CENTER,
                "Prism",
                FontId::proportional(13.5),
                p.text3,
            );
            Icon::ChevronUp.paint(
                ui,
                Rect::from_center_size(
                    egui::pos2(rect.right() - 13.5, rect.center().y),
                    egui::vec2(11.0, 11.0),
                ),
                p.muted,
            );
            popup::menu(&response, 252.0)
                .align(egui::RectAlign::TOP_START)
                .show(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    if popup::row_with_height(
                        ui,
                        Some(Icon::Gear),
                        "Settings",
                        if cfg!(target_os = "macos") {
                            "⌘,"
                        } else {
                            "Ctrl+,"
                        },
                        false,
                        32.0,
                    )
                    .clicked()
                    {
                        self.screen = Screen::Settings;
                        ui.close();
                    }
                    self.appearance_row(ui);
                    if popup::row_with_height(
                        ui,
                        Some(Icon::Retry),
                        "Run setup again",
                        "",
                        false,
                        32.0,
                    )
                    .clicked()
                    {
                        self.screen = Screen::Setup;
                        ui.close();
                    }
                    if popup::row_with_height(ui, Some(Icon::Star), "What’s new", "", false, 32.0)
                        .clicked()
                    {
                        self.news = Some(0);
                        ui.close();
                    }
                });
            self.notifications.button(ui);
            if widgets::icon_button(ui, Icon::Gear, "Settings", 34.0).clicked() {
                self.screen = Screen::Settings;
            }
        });
    }

    fn appearance_row(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::hover());
        Icon::System.paint(
            ui,
            Rect::from_center_size(
                rect.left_center() + egui::vec2(15.0, 0.0),
                egui::vec2(14.0, 14.0),
            ),
            p.text3,
        );
        ui.painter().text(
            rect.left_center() + egui::vec2(30.0, 0.0),
            Align2::LEFT_CENTER,
            "Appearance",
            FontId::proportional(13.5),
            p.text,
        );
        let switch = Rect::from_center_size(
            rect.right_center() - egui::vec2(51.0, 0.0),
            egui::vec2(86.0, 26.0),
        );
        ui.painter().rect_filled(switch, 8.0, p.fill);
        for (index, (name, icon)) in [
            ("Dark", Icon::Dark),
            ("Light", Icon::Light),
            ("System", Icon::System),
        ]
        .into_iter()
        .enumerate()
        {
            let bounds = Rect::from_min_size(
                switch.min + egui::vec2(2.0 + index as f32 * 28.0, 2.0),
                egui::vec2(26.0, 22.0),
            );
            if self.settings.appearance == name {
                ui.painter().rect_filled(bounds, 6.0, p.sel);
            }
            icon.paint(
                ui,
                Rect::from_center_size(bounds.center(), egui::vec2(14.0, 14.0)),
                p.muted,
            );
            let response = ui.interact(bounds, ui.id().with(name), Sense::click());
            if response.on_hover_text(name).clicked() {
                self.settings.appearance = name.into();
                self.persist_settings(ui.ctx());
            }
        }
    }
}
