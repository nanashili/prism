use super::app::Prism;
use super::footer_rows::{self, Lead, Tail};
use super::{
    agent_picker, icons::Icon, popup, settings_navigation::SettingsPage, theme::Palette, widgets,
};
use crate::model::Screen;
use crate::ui::presentation::{OpenTarget, SaveDestination};
use eframe::egui::{self, Align2, FontId, Rect, Sense};

#[derive(Clone, Copy)]
enum Section {
    Agents,
    Playbooks,
    Exports,
    Engine,
    Appearance,
}

impl Prism {
    pub(super) fn workbench_footer(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let p = Palette::for_context(ui.ctx());
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width() - 108.0, 36.0),
                Sense::click(),
            );
            if response.hovered() {
                ui.painter().rect_filled(rect, 8.0, p.sel);
            }
            footer_rows::paint_lead(
                ui,
                Lead::Prism,
                Rect::from_center_size(
                    rect.left_center() + egui::vec2(19.0, 0.0),
                    egui::vec2(22.0, 22.0),
                ),
            );
            ui.painter().text(
                rect.left_center() + egui::vec2(40.0, 0.0),
                Align2::LEFT_CENTER,
                "Prism",
                FontId::proportional(13.5),
                p.text3,
            );
            Icon::Disclosure.paint(
                ui,
                Rect::from_center_size(
                    rect.right_center() - egui::vec2(14.0, 0.0),
                    egui::vec2(12.0, 12.0),
                ),
                p.faint,
            );
            let mut frame = popup::surface(ui.ctx(), 6);
            frame.stroke.color = p.line3;
            frame.shadow.color = egui::Color32::from_black_alpha(115);
            popup::menu(&response, 260.0)
                .gap(8.0)
                .frame(frame)
                .align(egui::RectAlign::TOP_START)
                .show(|ui| self.workbench_footer_menu(ui));

            self.notifications.button(ui);
            if widgets::icon_button(ui, Icon::Search, "Search commands", 32.0).clicked() {
                self.palette = true;
            }
            if widgets::icon_button(ui, Icon::Gear, "Settings", 32.0).clicked() {
                self.open_settings_page(SettingsPage::General);
            }
        });
    }

    fn workbench_footer_menu(&mut self, ui: &mut egui::Ui) {
        popup::heading(ui, "Prism");
        if footer_rows::row(
            ui,
            Lead::Prism,
            &format!("Prism {}", env!("CARGO_PKG_VERSION")),
            "",
            Tail::Hint("•••"),
        )
        .clicked()
        {
            self.open_settings_page(SettingsPage::About);
            ui.close();
        }
        ui.add_space(8.0);
        for (section, label) in [
            (Section::Agents, "Agents"),
            (Section::Playbooks, "Playbooks"),
            (Section::Exports, "Exports"),
            (Section::Engine, "Engine"),
            (Section::Appearance, "Appearance"),
        ] {
            self.footer_submenu(ui, section, label);
        }
        if footer_rows::row(
            ui,
            Lead::Icon(Icon::Gear),
            "Settings",
            "",
            Tail::Hint(shortcut(",")),
        )
        .clicked()
        {
            self.open_settings_page(SettingsPage::General);
            ui.close();
        }
        ui.add_space(8.0);
        if footer_rows::row(
            ui,
            Lead::Icon(Icon::Folder),
            "Open a file",
            "",
            Tail::Hint(shortcut("O")),
        )
        .clicked()
        {
            self.choose_open(OpenTarget::File);
            ui.close();
        }
        if footer_rows::row(
            ui,
            Lead::Icon(Icon::Download),
            "Save project",
            "",
            Tail::Hint(shortcut("S")),
        )
        .clicked()
        {
            self.save_document(SaveDestination::Current);
            ui.close();
        }
    }

    fn footer_submenu(&mut self, ui: &mut egui::Ui, section: Section, label: &str) {
        let (lead, side) = match section {
            Section::Agents => (
                Lead::Agent(agent_picker::id(&self.settings)),
                agent_picker::label(&self.settings),
            ),
            Section::Playbooks => (Lead::Icon(Icon::Plan), ""),
            Section::Exports => (Lead::Icon(Icon::Download), ""),
            Section::Engine => (
                Lead::Engine(&self.settings.provider),
                match self.settings.provider.as_str() {
                    "ghidra-rust-rea" => "Ghidra (Rust through REA)",
                    "rizin" => "Rizin",
                    "ida" => "IDA",
                    "hopper" => "Hopper",
                    provider => provider,
                },
            ),
            Section::Appearance => (Lead::Icon(Icon::Dark), self.settings.appearance.as_str()),
        };
        let response = footer_rows::row(ui, lead, label, "", Tail::Submenu(side));
        ui.scope(|ui| {
            let p = Palette::for_context(ui.ctx());
            ui.visuals_mut().menu_corner_radius = egui::CornerRadius::same(12);
            ui.visuals_mut().window_stroke.color = p.line3;
            ui.visuals_mut().popup_shadow.color = egui::Color32::from_black_alpha(115);
            egui::containers::menu::SubMenu::new().show(ui, &response, |ui| {
                ui.set_width(276.0);
                match section {
                    Section::Agents => self.footer_agents(ui),
                    Section::Playbooks => self.footer_playbooks(ui),
                    Section::Exports => self.footer_exports(ui),
                    Section::Engine => self.footer_engines(ui),
                    Section::Appearance => self.footer_appearance(ui),
                }
            });
        });
    }

    pub(super) fn open_settings_page(&mut self, page: SettingsPage) {
        self.settings_ui.select(page);
        self.screen = Screen::Settings;
    }
}

fn shortcut(key: &str) -> &'static str {
    match (cfg!(target_os = "macos"), key) {
        (true, "O") => "⌘ O",
        (true, "S") => "⌘ S",
        (true, _) => "⌘ ,",
        (false, "O") => "Ctrl O",
        (false, "S") => "Ctrl S",
        (false, _) => "Ctrl ,",
    }
}
