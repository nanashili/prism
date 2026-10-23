mod progress;
pub(super) mod shimmer;
mod steps;

use super::{
    icons::Icon,
    message::{BodyCache, style::ChatStyle},
    tool_step,
};
use crate::model::{Activity, Block, Message, Outcome, now_ms};
use eframe::egui::{self, RichText};

pub(super) fn history<'a>(
    ui: &mut egui::Ui,
    message: &'a Message,
    thinking: bool,
    active: bool,
    style: &ChatStyle,
    cache: &mut BodyCache,
) -> &'a str {
    let Some(activity) = &message.activity else {
        return &message.text;
    };
    progress::show(ui, &activity.progress, thinking, style, cache);
    let end = activity
        .blocks
        .iter()
        .rposition(|block| !matches!(block, Block::Text { .. }))
        .map_or(0, |index| index + 1);
    let blocks = &activity.blocks[..end];
    let count = blocks
        .iter()
        .filter(|block| visible(block, thinking))
        .count();
    if count > 0 {
        let running = active && activity.outcome == Outcome::Running;
        let label = summary(activity, count, thinking);
        let id = ui.id().with("step-history");
        let open = shimmer::scope(ui, running, |ui| {
            tool_step::disclosure(ui, id, Icon::Thought, &label, "", running)
        });
        if open {
            steps::Page {
                activity,
                blocks,
                text: &message.text,
                thinking,
                running,
            }
            .show(ui, style, cache);
        }
        ui.add_space(8.0);
    }
    if activity.limited {
        ui.label(
            RichText::new("Showing recent activity. Older previews were shortened to save memory.")
                .small()
                .weak(),
        );
    }
    reply_text(message, activity, end)
}

fn summary(activity: &Activity, count: usize, thinking: bool) -> String {
    let omitted = activity.dropped_blocks.saturating_sub(if thinking {
        0
    } else {
        activity.dropped_thoughts
    });
    let count = count.saturating_add(omitted);
    let elapsed = duration(
        activity
            .ended
            .unwrap_or_else(now_ms)
            .saturating_sub(activity.started),
    );
    match activity.outcome {
        Outcome::Running => format!("Working for {elapsed}"),
        Outcome::Stopped => format!("Stopped after {count} steps"),
        _ if activity.ended.is_some() && activity.started > 0 => {
            format!("Worked through {count} steps in {elapsed}")
        }
        _ => format!("Worked through {count} steps"),
    }
}

fn reply_text<'a>(message: &'a Message, activity: &Activity, end: usize) -> &'a str {
    let start = activity.blocks[end..]
        .iter()
        .find_map(|block| match block {
            Block::Text { start, .. } => Some(*start),
            _ => None,
        })
        .unwrap_or_else(|| {
            activity
                .blocks
                .iter()
                .filter_map(|block| match block {
                    Block::Text { end, .. } => Some(*end),
                    _ => None,
                })
                .max()
                .unwrap_or(activity.dropped_text_end)
        });
    message.text.get(start..).unwrap_or(&message.text)
}

fn visible(block: &Block, thinking: bool) -> bool {
    thinking || !matches!(block, Block::Thought { .. })
}

pub(super) fn duration(milliseconds: i64) -> String {
    let seconds = milliseconds.max(0) / 1000;
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m {}s", seconds / 60, seconds % 60)
    }
}
