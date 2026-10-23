use super::{shimmer, visible};
use crate::model::{Activity, Block, Outcome};
use crate::ui::{
    message::{
        self, BodyCache,
        style::{BlockKind, ChatStyle},
    },
    tool_step,
};
use eframe::egui::{self, RichText};

pub(super) struct Page<'a> {
    pub activity: &'a Activity,
    pub blocks: &'a [Block],
    pub text: &'a str,
    pub thinking: bool,
    pub running: bool,
}

struct Window {
    id: egui::Id,
    start: usize,
    latest: usize,
    cursor: Option<usize>,
}

impl Page<'_> {
    pub fn show(&self, ui: &mut egui::Ui, style: &ChatStyle, cache: &mut BodyCache) {
        let window = Window::new(ui, self);
        window.earlier(ui, self);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (index, block) in self.visible().skip(window.start).take(32) {
                let active = self.running && index + 1 == self.activity.blocks.len();
                let position = self.activity.dropped_blocks.saturating_add(index);
                ui.push_id(position, |ui| {
                    step_block(ui, block, self.text, active, style, cache)
                });
            }
        });
        window.later(ui, self);
        if self.activity.outcome == Outcome::Failed {
            ui.label(
                RichText::new("The agent stopped with an error.")
                    .color(ui.visuals().error_fg_color),
            );
        }
    }

    fn visible(&self) -> impl Iterator<Item = (usize, &Block)> + Clone {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| visible(block, self.thinking))
    }
}

impl Window {
    fn new(ui: &egui::Ui, page: &Page<'_>) -> Self {
        let id = ui.id().with("step-page");
        let latest = page.visible().count().saturating_sub(32);
        let cursor = ui.data(|data| data.get_temp::<usize>(id));
        let start = cursor
            .map(|cursor| {
                page.visible()
                    .take_while(|(index, _)| {
                        page.activity.dropped_blocks.saturating_add(*index) < cursor
                    })
                    .count()
            })
            .unwrap_or(latest)
            .min(latest);
        Self {
            id,
            start,
            latest,
            cursor,
        }
    }

    fn earlier(&self, ui: &mut egui::Ui, page: &Page<'_>) {
        if self.start > 0
            && ui
                .small_button(format!("Show earlier steps ({})", self.start))
                .clicked()
            && let Some((index, _)) = page.visible().nth(self.start.saturating_sub(32))
        {
            let position = page.activity.dropped_blocks.saturating_add(index);
            ui.data_mut(|data| data.insert_temp(self.id, position));
        }
    }

    fn later(&self, ui: &mut egui::Ui, page: &Page<'_>) {
        if self.start < self.latest && ui.small_button("Show later steps").clicked() {
            let next = (self.start + 32).min(self.latest);
            ui.data_mut(|data| {
                if next == self.latest {
                    data.remove::<usize>(self.id);
                } else if let Some((index, _)) = page.visible().nth(next) {
                    let position = page.activity.dropped_blocks.saturating_add(index);
                    data.insert_temp(self.id, position);
                }
            });
        }
        if self.cursor.is_some() && ui.small_button("Latest steps").clicked() {
            ui.data_mut(|data| data.remove::<usize>(self.id));
        }
    }
}

fn step_block(
    ui: &mut egui::Ui,
    block: &Block,
    text: &str,
    active: bool,
    style: &ChatStyle,
    cache: &mut BodyCache,
) {
    match block {
        Block::Tool(tool) => tool_step::show(ui, tool, style),
        Block::Thought { text } => {
            style.block(ui, BlockKind::Thinking, |ui| {
                shimmer::scope(ui, active, |ui| step_text(ui, text, style, cache));
            });
        }
        Block::Text { start, end } => {
            if let Some(text) = text.get(*start..*end) {
                step_text(ui, text, style, cache);
            }
        }
    }
}

fn step_text(ui: &mut egui::Ui, text: &str, style: &ChatStyle, cache: &mut BodyCache) {
    if !text.trim().is_empty() {
        ui.add_space(4.0);
        message::render_body(ui, text, style, cache);
        ui.add_space(4.0);
    }
}
