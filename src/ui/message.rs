use crate::ui::presentation::{ActivityState, ChoiceAccess, ThoughtDisplay};
mod blocks;
mod body_cache;
pub(in crate::ui) mod choices;
pub(in crate::ui) mod layout;
mod markdown;
pub(in crate::ui) mod style;
pub(in crate::ui) use blocks::code_text;
pub(in crate::ui) use body_cache::BodyCache;

use crate::model::Message;
use eframe::egui::{self, Align, Frame, Layout};
use style::ChatStyle;

pub(super) fn render_message(
    ui: &mut egui::Ui,
    message: &Message,
    thinking: ThoughtDisplay,
    active: ActivityState,
    choices_allowed: ChoiceAccess,
    style: &ChatStyle,
) {
    render_message_cached(
        ui,
        message,
        thinking,
        active,
        choices_allowed,
        style,
        &mut BodyCache::default(),
    );
}

pub(super) fn render_message_cached(
    ui: &mut egui::Ui,
    message: &Message,
    thinking: ThoughtDisplay,
    active: ActivityState,
    choices_allowed: ChoiceAccess,
    style: &ChatStyle,
    cache: &mut BodyCache,
) {
    let choices_allowed = choices_allowed == ChoiceAccess::Allowed;

    cache.begin_row(ui.id());
    let user = message.role == "user";
    let response = ui.scope(|ui| {
        style.scope(ui, |ui| {
            let text = if user {
                super::playbooks::shown_text(&message.text)
            } else {
                super::activity_view::history(ui, message, thinking, active, style, cache).into()
            };
            if user {
                user_bubble(ui, &text, style, cache);
            } else if choices_allowed && let Some(choices) = choices::Choices::parse(&text) {
                render_body(ui, choices.intro, style, cache);
            } else if !text.trim().is_empty() {
                render_body(ui, &text, style, cache);
            }
        });
    });
    response.response.context_menu(|ui| {
        if ui.button("Copy message").clicked() {
            ui.ctx().copy_text(message.text.clone());
            ui.close();
        }
    });
    ui.add_space(style.appearance.message_spacing);
}

fn user_bubble(ui: &mut egui::Ui, text: &str, style: &ChatStyle, cache: &mut BodyCache) {
    let max_width = (ui.available_width() * 0.85 - 28.0).max(1.0);
    let measured = ui.fonts_mut(|fonts| {
        fonts.layout(
            preview_text(text).into(),
            style.font.clone(),
            style.palette.text,
            max_width,
        )
    });
    let width = measured.size().x.ceil().clamp(1.0, max_width);
    ui.with_layout(Layout::top_down(Align::Max), |ui| {
        Frame::NONE
            .fill(style.palette.fill2)
            .corner_radius(16)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(width);
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    render_body(ui, text, style, cache);
                });
            });
    });
}

pub(super) fn render_body(ui: &mut egui::Ui, text: &str, style: &ChatStyle, cache: &mut BodyCache) {
    let text = text.trim_matches(['\r', '\n']);
    let preview = preview_text(text);
    cache.show(ui, "markdown-preview", preview, style);
    if preview.len() < text.len() {
        ui.collapsing("Read full message", |ui| {
            cache.show(ui, "markdown-full", text, style);
        });
    }
}

fn preview_text(text: &str) -> &str {
    let mut end = text.len().min(16_000);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
