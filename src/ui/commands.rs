use super::app::Prism;
use crate::model::{Screen, View};
use crate::ui::presentation::{OpenTarget, TypingState};
use eframe::egui::{self, Key, Modifiers};

impl Prism {
    pub(super) fn shortcuts(&mut self, context: &egui::Context) {
        let editing_shortcuts = (self.screen == Screen::Settings
            && self.settings_ui.page == super::settings_navigation::SettingsPage::Keyboard)
            || (self.screen == Screen::Setup
                && self.settings_ui.setup_step == super::setup::KEYBOARD_STEP);
        if !editing_shortcuts {
            self.settings_ui.keyboard.recording = None;
        }
        let before = self.settings.views.shortcuts.clone();
        if self
            .settings_ui
            .keyboard
            .capture(context, &mut self.settings)
        {
            if self.settings.views.shortcuts != before {
                self.queue_settings_save(context);
            }
            return;
        }
        let typing = context.wants_keyboard_input();
        for &(id, _, _, default) in super::shortcut_catalog::COMMANDS {
            let global = matches!(
                id,
                "open" | "save" | "thread" | "palette" | "settings" | "stop"
            );
            if !global && self.screen != Screen::Workbench {
                continue;
            }
            if !super::shortcut_catalog::supported(id) || (typing && id == "undo") {
                continue;
            }
            let combo = super::shortcut_catalog::combo(&self.settings, id, default);
            if super::key_combo::pressed(context, combo, TypingState::from(typing)) {
                self.run_shortcut(id);
            }
        }
        self.workbench_tab_shortcuts(context);
        if !typing
            && context.input(|input| input.key_pressed(Key::Tab))
            && self.workspace.project.is_some()
        {
            let view = if self.workspace.view == View::Pseudocode {
                View::Assembly
            } else {
                View::Pseudocode
            };
            self.select_view(view);
        }
    }

    fn workbench_tab_shortcuts(&mut self, context: &egui::Context) {
        if self.screen != Screen::Workbench || self.workspace.project.is_none() {
            return;
        }
        if context.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::W)) {
            let active = self.workspace.active;
            self.close_workbench_tab(active);
        }
        let previous = context
            .input_mut(|input| input.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab));
        let next = context.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::Tab));
        if previous || next {
            let count = self.workspace.tabs.len();
            let active = self.workspace.active;
            let destination = if previous {
                (active + count - 1) % count
            } else {
                (active + 1) % count
            };
            self.select_workbench_tab(destination);
        }
    }

    pub(super) fn command_palette(&mut self, context: &egui::Context) {
        if !self.palette {
            return;
        }
        let mut open = true;
        egui::Window::new("Commands")
            .open(&mut open)
            .collapsible(false)
            .default_width(450.0)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .show(context, |ui| {
                let input = ui.text_edit_singleline(&mut self.command_filter);
                input.request_focus();
                let filter = self.command_filter.to_lowercase();
                for view in View::ALL {
                    if !self.view_relevant(view) {
                        continue;
                    }
                    if view.label().to_lowercase().contains(&filter)
                        && ui.button(view.label()).clicked()
                    {
                        self.screen = Screen::Workbench;
                        self.select_view(view);
                        self.palette = false;
                    }
                }
                if ui.button("Open file").clicked() {
                    self.choose_open(OpenTarget::File);
                    self.palette = false;
                }
                if ui.button("Setup").clicked() {
                    self.screen = Screen::Setup;
                    self.palette = false;
                }
            });
        if !open {
            self.palette = false;
        }
    }
}
