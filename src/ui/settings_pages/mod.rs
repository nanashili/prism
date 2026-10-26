use crate::ui::presentation::CatalogScope;
mod about;
mod agents;
mod chat;
mod code;
mod engines;
mod general;
pub(in crate::ui) mod model;
pub(in crate::ui) mod paths;
mod permissions;
mod presets;
mod render;
mod row;
mod row_control;
mod skills;
mod storage;

use crate::settings::Settings;
use crate::ui::{agent_catalog::Catalog, playbooks::Playbook, settings_navigation::SettingsPage};
use model::Group;
use std::sync::Arc;

pub(super) use presets::show as presets;
pub(super) use render::groups as show;
pub(super) use row_control::action_button;
pub(super) use skills::groups as skill_groups;

pub(super) struct PageContext<'a> {
    pub agents: &'a Catalog,
    pub playbooks: &'a [Arc<Playbook>],
    pub browsing_agents: bool,
}

pub(super) fn groups<'a>(
    page: SettingsPage,
    settings: &'a mut Settings,
    context: PageContext<'_>,
    custom_tool_server: &'a mut Option<String>,
) -> Vec<Group<'a>> {
    match page {
        SettingsPage::General => general::groups(settings),
        SettingsPage::Chat => chat::groups(settings),
        SettingsPage::Privacy => general::privacy(settings),
        SettingsPage::Agents => agents::groups(
            settings,
            context.agents,
            CatalogScope::from(context.browsing_agents),
            custom_tool_server,
        ),
        SettingsPage::Permissions => permissions::groups(settings),
        SettingsPage::Skills => skills::groups(settings, context.playbooks),
        SettingsPage::Engines => engines::groups(settings),
        SettingsPage::Code => code::groups(settings),
        SettingsPage::Debugging => code::debugging(settings),
        SettingsPage::Gpu => code::gpu(settings),
        SettingsPage::Storage => storage::groups(settings),
        SettingsPage::About => about::groups(),
        SettingsPage::Workspace | SettingsPage::Appearance | SettingsPage::Keyboard => Vec::new(),
    }
}
