use super::super::{
    app::Prism,
    icons::Icon,
    settings_navigation::SettingsPage,
    settings_pages::{self, model::Button, model::Command, model::Path},
    theme::Palette,
    widgets,
};
use crate::ui::presentation::{Border, Caret, ToolScope};
use crate::{model::Screen, settings::research::Category};
use eframe::egui::{self, Frame, RichText};

mod tools;

struct Tool {
    name: &'static str,
    description: &'static str,
    logo: egui::ImageSource<'static>,
    path: Path,
    url: &'static str,
    action_label: &'static str,
    automatic: bool,
}

impl Prism {
    pub(super) fn setup_engines(&mut self, ui: &mut egui::Ui) {
        if !self.settings.research.native_tools() {
            ui.label("Your agent and imported skills can help with web and protocol research. Native analysis tools are optional for these categories.");
            return;
        }

        let include_mobile = self.settings.research.contains(Category::Mobile);
        ui.label("Follow the REA installation instructions to download a prebuilt binary when available or build it from source, then choose the native executable below. Prism discovers a matching worker beside it when present.");
        ui.add_space(10.0);
        let p = Palette::for_context(ui.ctx());
        let selected = Frame::NONE
            .fill(p.well)
            .corner_radius(14)
            .stroke(egui::Stroke::new(1.0, p.line))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut selected = None;
                for (index, tool) in tools::for_workspace(ToolScope::from(include_mobile))
                    .iter()
                    .enumerate()
                {
                    if index > 0 {
                        tool_divider(ui, p);
                    }
                    let path = tool.path.value(&mut self.settings);
                    if let Some(command) = tool_row(ui, tool, path) {
                        selected = Some(command);
                    }
                }
                selected
            })
            .inner;

        if let Some(command) = selected {
            self.settings_command(ui.ctx(), command);
        }
        ui.add_space(12.0);
        if widgets::chip_tinted(
            ui,
            Some(Icon::Gear),
            "Engine settings",
            Border::Hidden,
            Caret::Hidden,
            p.muted,
        )
        .on_hover_text("Configure REA, Ghidra and memory limits")
        .clicked()
        {
            self.settings_ui.page = SettingsPage::Engines;
            self.screen = Screen::Settings;
        }
    }
}

fn tool_row(ui: &mut egui::Ui, tool: &Tool, path: &str) -> Option<Command> {
    Frame::NONE
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                ui.add(
                    egui::Image::new(tool.logo.clone()).fit_to_exact_size(egui::vec2(40.0, 40.0)),
                );

                let actions_width = 170.0;
                let text_width = ui.available_width() - actions_width - 14.0;
                ui.vertical(|ui| {
                    ui.set_width(text_width.max(120.0));
                    tool_details(ui, tool, path);
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    tool_actions(ui, tool)
                })
                .inner
            })
            .inner
        })
        .inner
}

fn tool_details(ui: &mut egui::Ui, tool: &Tool, path: &str) {
    let p = Palette::for_context(ui.ctx());
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.label(RichText::new(tool.name).size(15.0).strong().color(p.text));
    ui.label(RichText::new(tool.description).size(12.5).color(p.text3));

    let selected_path = std::path::Path::new(path);
    let label = if path.is_empty() || (matches!(tool.path, Path::Rea) && path == "rea") {
        if tool.automatic {
            "Automatic discovery".to_owned()
        } else {
            "Choose an installed executable".to_owned()
        }
    } else {
        let name = selected_path
            .file_name()
            .unwrap_or(selected_path.as_os_str());
        format!("Using {}", name.to_string_lossy())
    };
    let response =
        ui.add(egui::Label::new(RichText::new(label).size(11.5).color(p.muted)).truncate());
    if !path.is_empty() {
        response.on_hover_text(settings_pages::paths::display(selected_path));
    }
}

fn tool_actions(ui: &mut egui::Ui, tool: &Tool) -> Option<Command> {
    ui.spacing_mut().item_spacing.x = 8.0;
    let download = Button::new(tool.action_label, Command::Open(tool.url));
    let browse = Button::new("Browse…", Command::Choose(tool.path));

    if settings_pages::action_button(ui, &download)
        .on_hover_text(tool.url)
        .clicked()
    {
        return Some(download.command);
    }
    if settings_pages::action_button(ui, &browse)
        .on_hover_text(format!("Choose the {} executable", tool.name))
        .clicked()
    {
        return Some(browse.command);
    }
    None
}

fn tool_divider(ui: &mut egui::Ui, p: Palette) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().hline(
        rect.left() + 18.0..=rect.right() - 18.0,
        rect.center().y,
        (1.0, p.line),
    );
}
