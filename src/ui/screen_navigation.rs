use super::{app::Prism, settings_navigation::SettingsPage};
use crate::{
    error::{Result, failure},
    model::Screen,
};
use serde_json::Value;

impl Prism {
    pub(super) fn show_screen(&mut self, args: &Value) -> Result<()> {
        let screen = match args["screen"].as_str() {
            Some("Home") => Screen::Home,
            Some("Settings") => Screen::Settings,
            Some("Setup") => Screen::Setup,
            Some("Workbench") if self.workspace.project.is_some() => Screen::Workbench,
            Some("Workbench") => return Err(failure("Open a file before showing the workbench")),
            _ => return Err(failure("Choose Home, Settings, Setup, or Workbench")),
        };
        let page = match args.get("page") {
            None => None,
            Some(value) if screen == Screen::Settings => Some(
                SettingsPage::from_title(
                    value.as_str().ok_or("A settings page title is required")?,
                )
                .ok_or("Unknown settings page")?,
            ),
            Some(_) => {
                return Err(failure(
                    "A page can only be selected on the Settings screen",
                ));
            }
        };
        if let Some(page) = page {
            self.settings_ui.select(page);
        }
        self.screen = screen;
        Ok(())
    }
}
