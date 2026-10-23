use super::providers::{self, Provider};
use crate::settings::Settings;
use crate::ui::{agent_catalog::Catalog, agent_logos, popup, state::Chat, theme::Palette};
use eframe::egui;

pub(super) struct Selection {
    pub id: String,
    pub name: String,
    pub model: Option<String>,
}

pub(super) fn show(
    ui: &mut egui::Ui,
    catalog: &mut Catalog,
    settings: &Settings,
    chat: &Chat,
    selected: Option<&str>,
    opening: bool,
) -> Option<Selection> {
    let search = popup::search(ui, &mut catalog.query, "Search models and providers".into());
    if opening {
        search.request_focus();
    }
    ui.add_space(4.0);

    let query = catalog.query.trim().to_lowercase();
    let providers = providers::configured(settings, catalog);
    let mut picked = None;
    let mut shown = 0;
    egui::ScrollArea::vertical()
        .max_height(372.0)
        .show(ui, |ui| {
            for provider in &providers {
                if !matches(provider, &query) {
                    continue;
                }
                heading(ui, provider);
                ui.push_id(provider.id, |ui| {
                    let (count, selection) =
                        model_rows(ui, provider, &query, selected, chat.connected);
                    shown += count;
                    picked = selection.or(picked.take());
                });
            }
            if shown == 0 {
                let message = if providers.is_empty() {
                    "Set up an agent in Settings to see its models here."
                } else {
                    "No models match your search."
                };
                ui.label(egui::RichText::new(message).small().weak());
            }
        });
    picked
}

fn matches(provider: &Provider<'_>, query: &str) -> bool {
    provider.title.to_lowercase().contains(query)
        || provider.name.to_lowercase().contains(query)
        || provider
            .models
            .iter()
            .any(|model| model_matches(model, query))
}

fn model_matches(model: &crate::agent::options::Choice, query: &str) -> bool {
    model.name.to_lowercase().contains(query) || model.value.to_lowercase().contains(query)
}

fn model_rows(
    ui: &mut egui::Ui,
    provider: &Provider<'_>,
    query: &str,
    selected: Option<&str>,
    connected: bool,
) -> (usize, Option<Selection>) {
    if provider.models.is_empty() {
        let label = if provider.selected && connected {
            "Use agent default"
        } else {
            "Load models…"
        };
        let picked = popup::row(ui, None, label, provider.name, false).clicked();
        return (1, picked.then(|| selection(provider, None)));
    }
    let provider_matches = provider.title.to_lowercase().contains(query)
        || provider.name.to_lowercase().contains(query);
    let mut count = 0;
    let mut picked = None;
    for model in provider
        .models
        .iter()
        .filter(|model| provider_matches || model_matches(model, query))
    {
        count += 1;
        if count > 60 {
            continue;
        }
        let current = provider.selected && selected == Some(model.value.as_str());
        let response = popup::row(
            ui,
            None,
            &model.name,
            if current { "✓" } else { "" },
            current,
        );
        if response.on_hover_text(&model.description).clicked() {
            picked = Some(selection(provider, Some(model.value.clone())));
        }
    }
    if count > 60 {
        ui.label(
            egui::RichText::new("Type to narrow the model list.")
                .small()
                .weak(),
        );
    }
    (count, picked)
}

fn selection(provider: &Provider<'_>, model: Option<String>) -> Selection {
    Selection {
        id: provider.id.into(),
        name: provider.name.into(),
        model,
    }
}

fn heading(ui: &mut egui::Ui, provider: &Provider<'_>) {
    let p = Palette::for_context(ui.ctx());
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(15.0, 28.0), egui::Sense::hover());
        agent_logos::paint(
            ui,
            provider.logo,
            egui::Rect::from_center_size(rect.center(), egui::vec2(15.0, 15.0)),
        );
        ui.label(
            egui::RichText::new(provider.title)
                .size(12.0)
                .color(p.muted),
        );
    });
}
