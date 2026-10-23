use super::super::{app::Prism, icons::Icon, settings_navigation::SettingsPage, theme::Palette};
use crate::model::Screen;
use eframe::egui::{self, Frame, RichText};

impl Prism {
    pub(super) fn setup_engines(&mut self, ui: &mut egui::Ui) {
        if !self.settings.research.native_tools() {
            ui.label("Your agent and imported skills can help with web and protocol research. Native analysis tools are optional for these categories.");
            return;
        }
        self.setup_ghidra_tools(ui);

        ui.add_space(8.0);
        ui.label(RichText::new("Optional tools").strong());
        self.setup_optional_tools(ui);

        #[cfg(feature = "rea-rust-ghidra")]
        ui.hyperlink_to(
            "REA Rust Ghidra integration",
            "https://github.com/morluto/rea/tree/main/native/ghidra-rust-worker",
        );
        if ui.button("Configure engine and memory limits").clicked() {
            self.settings_ui.page = SettingsPage::Engines;
            self.screen = Screen::Settings;
        }
    }

    fn setup_ghidra_tools(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Ghidra through REA").strong());
        #[cfg(feature = "prism-ghidra-rea")]
        {
            ui.label("Prism's Java Ghidra provider uses this REA launcher. Configure your Ghidra installation and Java home in engine settings.");
            tool_row(
                ui,
                "REA for Java Ghidra, Hopper and IDA",
                &mut self.settings.rea,
                "https://github.com/morluto/rea",
                "Not found",
            );
        }
        #[cfg(feature = "rea-rust-ghidra")]
        {
            ui.label("Default Ghidra provider through REA. Choose the REA launcher and Rust worker paths when automatic discovery does not find them.");
            tool_row(
                ui,
                "REA launcher (Rust Ghidra)",
                &mut self.settings.tools.rust_rea,
                "https://github.com/morluto/rea",
                "Automatic",
            );
            tool_row(
                ui,
                "Rust Ghidra worker",
                &mut self.settings.tools.rust_ghidra_worker,
                "https://github.com/morluto/rea",
                "Automatic",
            );
        }
    }

    fn setup_optional_tools(&mut self, ui: &mut egui::Ui) {
        #[cfg(not(feature = "prism-ghidra-rea"))]
        tool_row(
            ui,
            "REA for Hopper and IDA",
            &mut self.settings.rea,
            "https://github.com/morluto/rea",
            "Not found",
        );
        tool_row(
            ui,
            "Rizin",
            &mut self.settings.rizin,
            "https://rizin.re",
            "Not found",
        );
        if self
            .settings
            .research
            .contains(crate::settings::research::Category::Mobile)
        {
            tool_row(
                ui,
                "jadx for Android",
                &mut self.settings.jadx,
                "https://github.com/skylot/jadx",
                "Not found",
            );
        }
    }
}

fn tool_row(ui: &mut egui::Ui, name: &str, path: &mut String, url: &str, empty_label: &str) {
    let p = Palette::for_context(ui.ctx());
    Frame::NONE
        .inner_margin(egui::Margin::symmetric(0, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                Icon::Model.show(ui, 32.0, p.muted);
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - 150.0).max(120.0));
                    ui.label(name);
                    let shown = if path.is_empty() {
                        empty_label
                    } else {
                        path.as_str()
                    };
                    ui.add(
                        egui::Label::new(RichText::new(shown).size(12.0).color(p.muted)).truncate(),
                    );
                });
                if ui.button("I have it").clicked()
                    && let Some(found) = rfd::FileDialog::new().pick_file()
                {
                    *path = found.to_string_lossy().into();
                }
                ui.hyperlink_to("Download", url);
            });
        });
}
