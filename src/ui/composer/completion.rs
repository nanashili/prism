use super::{
    mentions::Mention,
    palette_items::{Action, Item},
    picker::{self, Keys},
    query::Query,
};
use crate::ui::presentation::{InputFocus, Selection};
use crate::ui::{app::Prism, icons::Icon, popup};
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
    text_edit::TextEditOutput,
};

#[derive(Clone)]
pub(super) enum Suggestion {
    Palette(Item),
    Mention(Mention),
}

impl Suggestion {
    pub(super) fn row(&self, ui: &mut egui::Ui, selected: Selection) -> egui::Response {
        let selected = selected == Selection::Selected;

        match self {
            Self::Palette(item) => popup::row_with_height(
                ui,
                Some(item.icon),
                &item.label,
                &item.hint,
                Selection::from(selected),
                34.0,
            )
            .on_hover_text(&item.description),
            Self::Mention(mention) => {
                let icon = match mention.kind {
                    "bookmark" => Icon::Star,
                    "note" => Icon::Note,
                    _ => Icon::Function,
                };
                popup::row_with_height(
                    ui,
                    Some(icon),
                    &mention.label,
                    &mention.address,
                    Selection::from(selected),
                    34.0,
                )
            }
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct Completion {
    query: Option<Query>,
    dismissed: bool,
    selected: usize,
    entries: Vec<Suggestion>,
    loaded: bool,
}

impl Completion {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Self {
        ctx.data_mut(|data| data.remove_temp(id.with("completion")))
            .unwrap_or_default()
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|data| data.insert_temp(id.with("completion"), self));
    }

    pub fn is_open(&self) -> bool {
        self.query.is_some() && !self.dismissed
    }

    pub fn keys(&mut self, ui: &egui::Ui, focused: InputFocus) -> Keys {
        let focused = focused == InputFocus::Focused;

        let keys = picker::keys(
            ui,
            InputFocus::from(focused && self.is_open()),
            self.entries.len(),
            &mut self.selected,
        );
        if keys.close {
            self.dismissed = true;
        }
        keys
    }

    pub fn update(&mut self, draft: &str, cursor: Option<CCursorRange>, focused: InputFocus) {
        let focused = focused == InputFocus::Focused;

        let query = cursor.and_then(|cursor| Query::at(draft, cursor));
        if query != self.query {
            self.query = query;
            self.dismissed = false;
            self.selected = 0;
            self.entries.clear();
            self.loaded = false;
        }
        if !focused {
            self.dismissed = true;
        }
    }
}

impl Prism {
    pub(super) fn composer_suggestions(
        &mut self,
        ui: &egui::Ui,
        input: &mut TextEditOutput,
        completion: &mut Completion,
        keys: &Keys,
    ) {
        if !completion.is_open() {
            return;
        }
        if let Err(error) = self.refresh_suggestions(completion) {
            completion.dismissed = true;
            self.report_error(error);
            return;
        }
        let Some(query) = completion.query.as_ref() else {
            return;
        };
        let popup = egui::Popup::from_response(&input.response)
            .id(input.response.id.with("suggestions"))
            .align(egui::RectAlign::TOP_START)
            .width(input.response.rect.width().clamp(240.0, 440.0))
            .gap(8.0)
            .sense(egui::Sense::hover())
            .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
            .frame(popup::surface(ui.ctx(), 6))
            .show(|ui| {
                picker::rows(
                    ui,
                    &completion.entries,
                    completion.selected,
                    query.marker,
                    keys,
                )
            });
        let Some(popup) = popup else { return };
        if let Some(picked) = popup.inner {
            self.apply_completion(query, picked, input);
            completion.dismissed = true;
        } else if popup.response.clicked_elsewhere()
            && !ui.input(|i| {
                i.pointer
                    .interact_pos()
                    .is_some_and(|pos| input.response.rect.contains(pos))
            })
        {
            completion.dismissed = true;
        }
    }

    fn refresh_suggestions(&mut self, completion: &mut Completion) -> crate::error::Result<()> {
        let Some(query) = completion.query.as_ref() else {
            return Ok(());
        };
        if query.marker == '/' {
            if !completion.loaded {
                completion.loaded = true;
                self.prepare_agent_session();
            }
            completion.entries = self
                .palette_items(&query.text)
                .into_iter()
                .map(Suggestion::Palette)
                .collect();
        } else if !completion.loaded {
            completion.loaded = true;
            completion.entries = self
                .mention_items(&query.text)?
                .into_iter()
                .map(Suggestion::Mention)
                .collect();
        }
        completion.selected = completion
            .selected
            .min(completion.entries.len().saturating_sub(1));
        Ok(())
    }

    fn apply_completion(&mut self, query: &Query, picked: Suggestion, input: &mut TextEditOutput) {
        let replacement = match &picked {
            Suggestion::Palette(Item {
                action: Action::Command(name),
                ..
            }) => format!("/{name} "),
            Suggestion::Palette(Item {
                action: Action::Skill(skill),
                ..
            }) => format!("{} ", skill.reference()),
            Suggestion::Mention(mention) => format!("@{} ", mention.label),
            _ => String::new(),
        };
        let caret =
            self.chat.draft[..query.range.start].chars().count() + replacement.chars().count();
        self.chat
            .draft
            .replace_range(query.range.clone(), &replacement);
        match picked {
            Suggestion::Palette(Item {
                action: Action::Command(_) | Action::Skill(_),
                ..
            }) => {}
            Suggestion::Palette(Item {
                action: Action::Option(menu, value),
                ..
            }) => self.pick_option(&menu, value),
            Suggestion::Palette(Item {
                action: Action::Attach,
                ..
            }) => self.pick_attachments(),
            Suggestion::Mention(mention) => {
                self.chat
                    .mentions
                    .retain(|previous| previous.label != mention.label);
                if self.chat.mentions.len() >= 32 {
                    self.chat.mentions.remove(0);
                }
                self.chat.mentions.push(mention);
            }
        }
        input
            .state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(
                caret.min(self.chat.draft.chars().count()),
            ))));
        input
            .state
            .clone()
            .store(&input.response.ctx, input.response.id);
        input.response.request_focus();
        input.response.ctx.request_repaint();
    }
}
