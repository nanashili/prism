use super::app::Prism;
use super::footer_rows::{self, Lead, Tail};
use super::{icons::Icon, popup, settings_navigation::SettingsPage, theme::Palette, widgets};
use eframe::egui::{self, Align2, FontId, Rect, Sense};

impl Prism {
    pub(super) fn footer_engines(&mut self, ui: &mut egui::Ui) {
        popup::heading(ui, "Reads the file");
        ui.add_enabled_ui(self.pending == 0 && !self.chat.busy, |ui| {
            for (id, name) in [
                #[cfg(all(
                    feature = "rea-rust-ghidra",
                    any(target_os = "macos", target_os = "linux")
                ))]
                ("ghidra-rust-rea", "Ghidra (Rust through REA)"),
                #[cfg(feature = "prism-ghidra-rea")]
                ("ghidra-rea", "Ghidra (Java through REA)"),
                ("rizin", "Rizin"),
                ("ida", "IDA"),
                ("hopper", "Hopper"),
            ] {
                let selected = self.settings.provider == id;
                let note = if selected {
                    "Selected"
                } else {
                    "Use for the next analysis request"
                };
                if footer_rows::row(ui, Lead::Engine(id), name, note, Tail::Check(selected))
                    .clicked()
                {
                    if !selected {
                        self.settings.provider = id.into();
                        self.persist_settings(ui.ctx());
                    }
                    ui.close();
                }
            }
        });
        ui.add_space(8.0);
        if footer_rows::row(
            ui,
            Lead::Icon(Icon::Gear),
            "Engine settings",
            "",
            Tail::None,
        )
        .clicked()
        {
            self.open_settings_page(SettingsPage::Engines);
            ui.close();
        }
    }

    pub(super) fn footer_appearance(&mut self, ui: &mut egui::Ui) {
        popup::heading(ui, "Theme");
        for (id, label, icon) in [
            ("Dark", "Dark", Icon::Dark),
            ("Light", "Light", Icon::Light),
            ("System", "Match the system", Icon::System),
        ] {
            if footer_rows::row(
                ui,
                Lead::Icon(icon),
                label,
                "",
                Tail::Check(self.settings.appearance == id),
            )
            .clicked()
            {
                self.settings.appearance = id.into();
                self.persist_settings(ui.ctx());
            }
        }
        ui.add_space(8.0);
        popup::heading(ui, "Code");
        self.footer_font_size(ui);
    }

    fn footer_font_size(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::hover());
        Icon::Text.paint(
            ui,
            Rect::from_center_size(
                rect.left_center() + egui::vec2(16.5, 0.0),
                egui::vec2(17.0, 17.0),
            ),
            p.text3,
        );
        ui.painter().text(
            rect.left_center() + egui::vec2(35.0, 0.0),
            Align2::LEFT_CENTER,
            "Font size",
            FontId::proportional(13.5),
            p.text3,
        );
        let control = Rect::from_center_size(
            rect.right_center() - egui::vec2(52.0, 0.0),
            egui::vec2(92.0, 26.0),
        );
        ui.painter().rect_filled(control, 8.0, p.fill2);
        let mut controls_ui =
            ui.new_child(egui::UiBuilder::new().max_rect(control.shrink2(egui::vec2(2.0, 2.0))));
        controls_ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let smaller = ui
                .add_enabled_ui(self.settings.code_size > 9.0, |ui| {
                    widgets::icon_button(ui, Icon::ZoomOut, "Smaller", 22.0)
                })
                .inner
                .clicked();
            let (label, _) = ui.allocate_exact_size(egui::vec2(36.0, 22.0), Sense::hover());
            ui.painter().text(
                label.center(),
                Align2::CENTER_CENTER,
                format!("{} pt", self.settings.code_size),
                FontId::proportional(12.0),
                p.text3,
            );
            let larger = ui
                .add_enabled_ui(self.settings.code_size < 20.0, |ui| {
                    widgets::icon_button(ui, Icon::ZoomIn, "Larger", 22.0)
                })
                .inner
                .clicked();
            if smaller || larger {
                self.settings.code_size += if smaller { -1.0 } else { 1.0 };
                self.queue_settings_save(ui.ctx());
            }
        });
    }
}
