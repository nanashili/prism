use crate::model::{Child, TurnProgress};
use crate::ui::presentation::{Disclosure, ThoughtDisplay};
use crate::ui::{icons::Icon, message, theme::Palette, tool_step};
use eframe::egui;
use message::style::{BlockKind, ChatStyle};

pub(super) fn show(
    ui: &mut egui::Ui,
    progress: &TurnProgress,
    thinking: ThoughtDisplay,
    style: &ChatStyle,
    cache: &mut message::BodyCache,
) {
    let thinking = thinking == ThoughtDisplay::Visible;

    show_plan(ui, progress, style);
    show_children(ui, progress, thinking, style, cache);
    if progress.limited {
        ui.weak("Showing recent subagent activity. Older previews were shortened to save memory.");
    }
}

fn show_plan(ui: &mut egui::Ui, progress: &TurnProgress, style: &ChatStyle) {
    if !progress.plan.is_empty() {
        let done = progress
            .plan
            .iter()
            .filter(|item| item.status == "completed")
            .count();
        let label = format!("Plan · {done} of {} done", progress.plan.len());
        if tool_step::disclosure(
            ui,
            ui.id().with("plan"),
            Icon::Plan,
            &label,
            "",
            Disclosure::Open,
        ) {
            ui.indent("plan-items", |ui| {
                for item in &progress.plan {
                    ui.horizontal_top(|ui| {
                        let icon = match item.status.as_str() {
                            "completed" => Icon::Check,
                            "in_progress" => Icon::Spark,
                            _ => Icon::Ask,
                        };
                        icon.show(ui, 13.0, Palette::for_context(ui.ctx()).muted);
                        let text = style.text(&item.content);
                        ui.add(
                            egui::Label::new(if item.status == "completed" {
                                text.weak().strikethrough()
                            } else {
                                text
                            })
                            .wrap(),
                        );
                    });
                }
            });
        }
        ui.add_space(6.0);
    }
}

fn show_children(
    ui: &mut egui::Ui,
    progress: &TurnProgress,
    thinking: bool,
    style: &ChatStyle,
    cache: &mut message::BodyCache,
) {
    if !progress.children.is_empty() {
        let working = progress
            .children
            .iter()
            .filter(|child| matches!(child.status.as_str(), "running" | "waiting"))
            .count();
        let label = format!(
            "Subagents · {} · {working} working",
            progress.children.len()
        );
        if tool_step::disclosure(
            ui,
            ui.id().with("children"),
            Icon::Thread,
            &label,
            "",
            Disclosure::Closed,
        ) {
            ui.indent("child-agents", |ui| {
                for child in &progress.children {
                    ui.push_id(&child.id, |ui| {
                        let parent = child
                            .parent
                            .as_ref()
                            .and_then(|id| progress.children.iter().find(|other| &other.id == id));
                        let title = parent.map_or_else(
                            || child.name.clone(),
                            |parent| format!("{} / {}", parent.name, child.name),
                        );
                        show_child(ui, child, &title, thinking, style, cache);
                    });
                }
            });
        }
        ui.add_space(6.0);
    }
}

fn show_child(
    ui: &mut egui::Ui,
    child: &Child,
    title: &str,
    thinking: bool,
    style: &ChatStyle,
    cache: &mut message::BodyCache,
) {
    if !tool_step::disclosure(
        ui,
        ui.id().with("child"),
        Icon::Spark,
        title,
        status(&child.status),
        Disclosure::Closed,
    ) {
        return;
    }
    if !child.task.is_empty() {
        ui.weak(&child.task);
    }
    if thinking && !child.thought.is_empty() {
        ui.collapsing("Thinking", |ui| {
            style.block(ui, BlockKind::Thinking, |ui| {
                message::render_body(ui, &child.thought, style, cache);
            });
        });
    }
    for tool in &child.tools {
        tool_step::show(ui, tool, style);
    }
    if !child.text.is_empty() {
        message::render_body(ui, &child.text, style, cache);
    }
}

fn status(status: &str) -> &str {
    match status {
        "running" => "Working",
        "waiting" => "Needs approval",
        "done" => "Finished",
        "cancelled" => "Stopped",
        "failed" => "Failed",
        "disconnected" => "Disconnected",
        _ => "Activity unknown",
    }
}
