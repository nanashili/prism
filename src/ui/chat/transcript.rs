mod identities;
mod rows;
mod viewport;

pub(in crate::ui) use identities::MessageIdentities;

use crate::ui::{
    message::{BodyCache, style::ChatStyle},
    state::Chat,
};
use eframe::egui;
use rows::{RowSpec, Rows};

const MAX_PAGES: usize = 8;

#[derive(Default)]
pub(in crate::ui) struct TranscriptView {
    pages: Vec<PageView>,
    bodies: BodyCache,
}

struct PageView {
    id: egui::Id,
    rows: Rows,
}

impl TranscriptView {
    pub fn clear_bodies(&mut self) {
        self.bodies.clear();
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        chat: &Chat,
        tab: u64,
        thinking: bool,
        style: &ChatStyle,
    ) {
        let history = chat.history.page.as_ref();
        let page_id = history.and_then(|page| page.messages.first()?.id);
        let id = egui::Id::new(("transcript-page", tab, history.is_some(), page_id));
        let mut page = self.take_page(id);
        let specs = row_specs(ui, chat, id, style);
        // UiBuilder::id hashes this salt again to form the row's global UI ID.
        let rows: Vec<_> = specs.iter().map(|spec| egui::Id::new(spec.id)).collect();
        self.bodies.begin_frame(id, &rows);
        page.show(
            ui,
            viewport::RenderFrame {
                chat,
                thinking,
                style,
                cache: &mut self.bodies,
                specs: &specs,
            },
        );
        self.pages.push(page);
        if self.pages.len() > MAX_PAGES {
            self.pages.remove(0);
        }
    }

    fn take_page(&mut self, id: egui::Id) -> PageView {
        self.pages
            .iter()
            .position(|page| page.id == id)
            .map(|index| self.pages.remove(index))
            .unwrap_or_else(|| PageView {
                id,
                rows: Rows::default(),
            })
    }
}

fn row_specs(ui: &egui::Ui, chat: &Chat, page: egui::Id, style: &ChatStyle) -> Vec<RowSpec> {
    let history = chat.history.page.as_ref();
    let messages = history.map_or(chat.messages.as_slice(), |page| &page.messages);
    let width = (ui.available_width() - 32.0).max(1.0);
    let mut specs = Vec::with_capacity(messages.len() + usize::from(chat.busy));
    for (index, message) in messages.iter().enumerate() {
        let identity = if history.is_some() {
            egui::Id::new(("stored", message.id, index))
        } else {
            egui::Id::new(("live", chat.transcript_ids.get(index)))
        };
        let characters_per_line = (width / (style.font.size * 0.5)).max(1.0);
        let lines = (message.text.len().min(16_000) as f32 / characters_per_line)
            .ceil()
            .max(1.0);
        specs.push(RowSpec {
            id: page.with(identity),
            estimate: lines * style.appearance.line_height(style.font.size)
                + style.appearance.message_spacing
                + if message.activity.is_some() {
                    40.0
                } else {
                    20.0
                },
        });
    }
    if chat.busy {
        specs.push(RowSpec {
            id: page.with("status"),
            estimate: style.font.size * 1.5,
        });
    }
    specs
}
