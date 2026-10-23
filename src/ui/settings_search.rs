use super::playbooks::Playbook;
use super::{agent_catalog::Catalog, settings_navigation::SettingsPage, settings_pages};
use crate::settings::Settings;
use std::sync::Arc;

pub(super) fn page_matches(
    page: SettingsPage,
    query: &str,
    settings: &mut Settings,
    catalog: &Catalog,
    playbooks: &[Arc<Playbook>],
) -> bool {
    if query.is_empty() {
        return true;
    }
    match page {
        SettingsPage::Skills => "skills library libraries download github repository import folder archives".contains(query)
            || playbooks.iter().any(|entry| format!("{} {}", entry.title, entry.description).to_lowercase().contains(query)),
        SettingsPage::Workspace => "workspace categories interests binaries firmware hardware web mobile network protocols games engines runtime memory security graphics models".contains(query),
        SettingsPage::Appearance => {
            "appearance theme dark light match computer interface size window transparency frost code font colours line spacing numbers graphs"
                .contains(query)
        }
        SettingsPage::Keyboard => super::shortcut_catalog::COMMANDS.iter().any(
            |(_, label, group, _)| {
                format!("{label} {group} keyboard shortcuts keymap import")
                    .to_lowercase()
                    .contains(query)
            },
        ),
        _ => settings_pages::groups(
            page,
            settings,
            settings_pages::PageContext {
                agents: catalog,
                playbooks,
                browsing_agents: false,
            },
            &mut None,
        )
            .iter()
            .any(|group| {
                group
                    .rows
                    .iter()
                    .any(|row| row_matches(row, group.label, page, query))
            }),
    }
}

pub(super) fn row_matches(
    row: &settings_pages::model::Row<'_>,
    group: &str,
    page: SettingsPage,
    query: &str,
) -> bool {
    query.is_empty()
        || format!("{} {} {group} {}", row.title, row.description, page.title())
            .to_lowercase()
            .contains(query)
}
