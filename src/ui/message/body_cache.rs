use super::{layout::LayoutKey, markdown, style::ChatStyle};
use crate::ui::presentation::ThoughtDisplay;
use eframe::egui;
use egui_commonmark_extended::CommonMarkCache;

const MAX_ENTRIES: usize = 32;
const MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_BODY_BYTES: usize = 64 * 1024;

#[derive(Default)]
pub(in crate::ui) struct BodyCache {
    entries: Vec<Body>,
    page: Option<egui::Id>,
    row: Option<egui::Id>,
    clock: u64,
    source_bytes: usize,
}

struct Body {
    id: egui::Id,
    row: egui::Id,
    source: Box<str>,
    normalized: Option<Box<str>>,
    syntax_hints: bool,
    layout: LayoutKey,
    cache: CommonMarkCache,
    last_used: u64,
}

impl BodyCache {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.page = None;
        self.row = None;
        self.source_bytes = 0;
    }

    pub fn begin_frame(&mut self, page: egui::Id, rows: &[egui::Id]) {
        if self.page != Some(page) {
            self.clear();
            self.page = Some(page);
        }
        self.entries.retain(|body| rows.contains(&body.row));
        self.recount_sources();
    }

    pub fn begin_row(&mut self, row: egui::Id) {
        self.row = Some(row);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, role: &str, text: &str, style: &ChatStyle) {
        let id = ui.id().with(role);
        let existing = self.entries.iter().position(|body| body.id == id);
        if text.len() > MAX_BODY_BYTES {
            if let Some(index) = existing {
                self.remove(index);
            }
            markdown::show(ui, text, style);
            return;
        }

        let layout = LayoutKey::new(ui, style, ThoughtDisplay::Hidden);
        let index = match existing {
            Some(index) if self.entries[index].matches(text, style) => index,
            previous => {
                if let Some(index) = previous {
                    self.remove(index);
                }
                let body = Body::new(id, self.row.unwrap_or(ui.id()), text, style, layout);
                self.admit(body)
            }
        };
        self.clock += 1;
        let body = &mut self.entries[index];
        body.last_used = self.clock;
        body.show(ui, style, layout);
    }

    fn admit(&mut self, body: Body) -> usize {
        let bytes = body.source_bytes();
        while self.entries.len() >= MAX_ENTRIES || self.source_bytes + bytes > MAX_SOURCE_BYTES {
            let oldest = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, body)| body.last_used)
                .map(|(index, _)| index);
            let Some(index) = oldest else {
                break;
            };
            self.remove(index);
        }
        self.source_bytes += bytes;
        self.entries.push(body);
        self.entries.len() - 1
    }

    fn recount_sources(&mut self) {
        // This ledger covers owned source only, not the backend's private heap.
        self.source_bytes = self.entries.iter().map(Body::source_bytes).sum();
    }

    fn remove(&mut self, index: usize) {
        self.source_bytes -= self.entries.swap_remove(index).source_bytes();
    }
}

impl Body {
    fn new(id: egui::Id, row: egui::Id, text: &str, style: &ChatStyle, layout: LayoutKey) -> Self {
        let normalized = (!style.appearance.block_colors)
            .then(|| markdown::without_syntax_hints(text))
            .and_then(|text| match text {
                std::borrow::Cow::Owned(text) => Some(text.into_boxed_str()),
                std::borrow::Cow::Borrowed(_) => None,
            });
        Self {
            id,
            row,
            source: text.into(),
            normalized,
            syntax_hints: style.appearance.block_colors,
            layout,
            cache: CommonMarkCache::default(),
            last_used: 0,
        }
    }

    fn matches(&self, text: &str, style: &ChatStyle) -> bool {
        self.source.as_ref() == text && self.syntax_hints == style.appearance.block_colors
    }

    fn source_bytes(&self) -> usize {
        self.source.len() + self.normalized.as_ref().map_or(0, |text| text.len())
    }

    fn show(&mut self, ui: &mut egui::Ui, style: &ChatStyle, layout: LayoutKey) {
        let rendered = self.normalized.as_deref().unwrap_or(&self.source);
        if self.layout != layout {
            markdown::reset_layout(&mut self.cache, rendered);
            self.layout = layout;
        }
        markdown::show_cached(ui, rendered, style, &mut self.cache);
    }
}
