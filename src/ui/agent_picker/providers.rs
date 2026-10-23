use super::custom_id;
use crate::agent::{options::Choice, picks};
use crate::settings::Settings;
use crate::ui::agent_catalog::Catalog;

pub(super) struct Provider<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub logo: &'a str,
    pub title: &'a str,
    pub models: &'a [Choice],
    pub selected: bool,
}

pub(super) fn configured<'a>(settings: &'a Settings, catalog: &'a Catalog) -> Vec<Provider<'a>> {
    let custom = !settings.agent.trim().is_empty();
    let custom_logo = custom_id(settings);
    let hidden = &settings.conversation.hidden_agents;
    let mut providers = Vec::new();
    for entry in &catalog.entries {
        if !catalog.installed.contains(&entry.id)
            || hidden.contains(&entry.id)
            || (custom && custom_logo == entry.id)
        {
            continue;
        }
        providers.push(Provider {
            id: &entry.id,
            name: &entry.name,
            logo: &entry.id,
            title: title(&entry.id, &entry.name),
            models: catalog.models.get(&entry.id).map_or(&[], Vec::as_slice),
            selected: settings.agent_id == entry.id,
        });
    }
    if custom && !hidden.iter().any(|id| id == custom_logo) {
        let selected = settings.agent_id.is_empty() || settings.agent_id == custom_logo;
        let key = picks::command_key(&settings.agent, &settings.agent_args);
        let name = crate::ui::agent_options::short_agent(&settings.agent, &settings.agent_args);
        let logo = custom_logo;
        providers.push(Provider {
            id: if selected { &settings.agent_id } else { "" },
            name,
            logo,
            title: title(logo, name),
            models: catalog
                .models
                .get(&key)
                .or_else(|| catalog.models.get(logo))
                .map_or(&[], Vec::as_slice),
            selected,
        });
    }
    providers.sort_by(|a, b| a.title.cmp(b.title).then(a.name.cmp(b.name)));
    providers
}

pub(super) fn title<'a>(id: &str, name: &'a str) -> &'a str {
    match id {
        "claude-acp" => "Anthropic",
        "codex-acp" => "ChatGPT",
        "gemini" => "Google",
        _ => name,
    }
}
