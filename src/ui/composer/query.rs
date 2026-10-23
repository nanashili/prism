use eframe::egui::text::CCursorRange;
use std::ops::Range;

#[derive(Clone, PartialEq)]
pub(super) struct Query {
    pub range: Range<usize>,
    pub text: String,
    pub marker: char,
}

impl Query {
    pub fn at(draft: &str, cursor: CCursorRange) -> Option<Self> {
        if !cursor.is_empty() {
            return None;
        }
        let caret = draft
            .char_indices()
            .nth(cursor.primary.index)
            .map_or(draft.len(), |(i, _)| i);
        let prefix = &draft[..caret];
        let start = prefix
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace())
            .map_or(0, |(i, c)| i + c.len_utf8());
        let token = &draft[start..caret];
        let marker = token.chars().next()?;
        if !matches!(marker, '/' | '@') || token.len() > 129 {
            return None;
        }
        if marker == '/' && !draft[..start].trim().is_empty() {
            return None;
        }
        let end = draft[caret..]
            .char_indices()
            .find(|(_, c)| c.is_whitespace())
            .map_or(draft.len(), |(i, _)| caret + i);
        Some(Self {
            range: start..end,
            text: token[1..].into(),
            marker,
        })
    }
}
