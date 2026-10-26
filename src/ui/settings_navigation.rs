use super::app::Prism;
use super::{icons::Icon, theme::Palette, widgets};
use crate::model::Screen;
use crate::ui::presentation::Selection;
use eframe::egui::{self, Frame};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SettingsPage {
    #[default]
    General,
    Workspace,
    Appearance,
    Keyboard,
    Chat,
    Agents,
    Permissions,
    Privacy,
    Skills,
    Engines,
    Code,
    Debugging,
    Gpu,
    Storage,
    About,
}

#[derive(Default)]
pub(super) struct SettingsUi {
    pub page: SettingsPage,
    pub libraries: super::skill_libraries::LibraryUi,
    pub setup_step: usize,
    pub pending_save: Option<std::time::Instant>,
    pub query: String,
    pub confirm: Option<super::settings_pages::model::Command>,
    pub browsing_agents: bool,
    pub keyboard: super::keyboard_edit::Editor,
    pub custom_tool_server: Option<String>,
    pub storage: super::settings_storage::StorageUi,
}

impl SettingsUi {
    pub fn select(&mut self, page: SettingsPage) {
        if page == SettingsPage::Storage {
            self.storage.invalidate();
        }
        self.page = page;
        self.query.clear();
        self.confirm = None;
        self.keyboard.recording = None;
    }
}

pub(super) const SETTINGS_GROUPS: &[&[SettingsPage]] = &{
    use SettingsPage::{
        About, Agents, Appearance, Chat, Code, Debugging, Engines, General, Gpu, Keyboard,
        Permissions, Privacy, Skills, Storage, Workspace,
    };
    [
        &[General, Workspace, Appearance, Keyboard],
        &[Chat, Agents, Permissions, Privacy, Skills],
        &[Engines, Code, Debugging, Gpu],
        &[Storage, About],
    ]
};

impl SettingsPage {
    pub fn from_title(title: &str) -> Option<Self> {
        SETTINGS_GROUPS
            .iter()
            .flat_map(|group| group.iter())
            .copied()
            .find(|page| page.title().eq_ignore_ascii_case(title))
    }

    pub fn relevant(self, settings: &crate::settings::Settings) -> bool {
        match self {
            Self::Engines | Self::Code => settings.research.native_tools(),
            Self::Debugging => settings.research.debugger(),
            Self::Gpu => settings
                .research
                .contains(crate::settings::research::Category::Graphics),
            _ => true,
        }
    }

    pub fn title(self) -> &'static str {
        self.metadata().0
    }

    pub fn intro(self) -> &'static str {
        self.metadata().1
    }

    fn metadata(self) -> (&'static str, &'static str) {
        match self {
            Self::Workspace => (
                "Workspace",
                "Categories, tools and skills for the work you do.",
            ),
            Self::General => ("General", "How Prism starts, updates and keeps you posted."),
            Self::Appearance => (
                "Appearance",
                "Theme, sizes and how code looks everywhere in Prism.",
            ),
            Self::Keyboard => (
                "Keyboard",
                "Click a shortcut to change it. Shortcuts follow the tool your hands already know.",
            ),
            Self::Chat => (
                "Chat",
                "How conversations look and when Prism gets your attention.",
            ),
            Self::Agents => (
                "Agents",
                "The agents on this Mac, which one starts new projects, and how they run.",
            ),
            Self::Permissions => (
                "Permissions",
                "What agents can do on their own and what they ask you about first.",
            ),
            Self::Privacy => (
                "Privacy",
                "What stays on this Mac and what goes to the agent you use.",
            ),
            Self::Skills => (
                "Skills",
                "Playbooks you can start from the composer or the palette. Any agent can follow them.",
            ),
            Self::Engines => (
                "Engines",
                "The tools Prism reads apps with. One is enough. With more, Prism picks the right one for each job.",
            ),
            Self::Code => ("Code views", "Pseudocode, assembly, graphs, hex and lists."),
            Self::Debugging => (
                "Debugging",
                "Running a program under the debugger from the workbench.",
            ),
            Self::Gpu => (
                "GPU",
                "Recording GPU traces for shaders, models and Metal apps.",
            ),
            Self::Storage => (
                "Storage",
                "Where Prism keeps your work and how much room it takes.",
            ),
            Self::About => ("About", "Version, updates and help."),
        }
    }
}

impl Prism {
    pub(super) fn settings_navigation(&mut self, ui: &mut egui::Ui) {
        ui.add_space(46.0);
        if widgets::row(ui, Icon::Back, "Back", Selection::Unselected, 30.0).clicked() {
            self.screen = if self.workspace.project.is_some() {
                Screen::Workbench
            } else {
                Screen::Home
            };
        }
        ui.add_space(8.0);
        let p = Palette::for_context(ui.ctx());
        Frame::NONE
            .fill(p.card)
            .stroke(egui::Stroke::new(1.0, p.line))
            .corner_radius(10)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Icon::Search.show(ui, 14.0, p.faint);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.settings_ui.query)
                            .frame(false)
                            .char_limit(200)
                            .hint_text("Search Settings")
                            .desired_width(ui.available_width()),
                    );
                    if response.changed() {
                        self.settings_ui.confirm = None;
                    }
                });
            });
        ui.add_space(14.0);
        self.settings_page_list(ui);
    }

    fn settings_page_list(&mut self, ui: &mut egui::Ui) {
        let needle = self.settings_ui.query.trim().to_lowercase();
        let playbooks = if needle.is_empty() {
            Vec::new()
        } else {
            self.playbooks.all(&self.settings)
        };
        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                for group in SETTINGS_GROUPS {
                    for &page in *group {
                        let title = page.title();
                        if needle.is_empty() && !page.relevant(&self.settings) {
                            continue;
                        }
                        if !super::settings_search::page_matches(
                            page,
                            &needle,
                            &mut self.settings,
                            &self.agent_catalog,
                            &playbooks,
                        ) {
                            continue;
                        }
                        if super::settings_page_row::show(
                            ui,
                            page,
                            title,
                            Selection::from(self.settings_ui.page == page),
                        )
                        .clicked()
                        {
                            self.settings_ui.select(page);
                        }
                        ui.add_space(2.0);
                    }
                    ui.add_space(14.0);
                }
            });
    }
}
