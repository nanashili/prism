use crate::ui::{
    agent_logos, agent_options, agent_picker,
    app::{Chat, Prism},
    tab_strip,
    theme::Palette,
};
use eframe::egui::{self, FontId, Rect};
use std::borrow::Cow;

enum TabAction {
    Select(u64),
    Close(u64),
}

impl Prism {
    pub(super) fn agent_chat_tabs(&mut self, ui: &mut egui::Ui) -> Rect {
        let active = self.chat_tabs.active;
        let changed = ui.ctx().data_mut(|data| {
            let id = ui.id().with("visible-chat-tab");
            let previous = data.get_temp::<u64>(id);
            data.insert_temp(id, active);
            previous != Some(active)
        });
        let mut action = None;
        let controls = tab_strip::show(ui, 128.0, |ui| {
            if self.chat_tabs.inspector {
                let rect = ui.available_rect_before_wrap();
                ui.painter().text(
                    rect.left_center() + egui::vec2(8.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    "Inspector",
                    FontId::proportional(13.0),
                    Palette::for_context(ui.ctx()).text3,
                );
                return;
            }
            for key in &self.chat_tabs.order {
                if let Some(command) = self.chat_tab_item(ui, *key, changed) {
                    action = Some(command);
                }
            }
        });
        match action {
            Some(TabAction::Select(key)) => self.select_chat_tab(key),
            Some(TabAction::Close(key)) => self.close_chat_tab(key),
            None => {}
        }
        controls
    }

    fn chat_tab_item(&self, ui: &mut egui::Ui, key: u64, reveal: bool) -> Option<TabAction> {
        let (chat, logo) = self.chat_tab_content(key)?;
        let title = if chat.title.is_empty() {
            "New chat"
        } else {
            &chat.title
        };
        let active = key == self.chat_tabs.active;
        let item = ui
            .push_id(("chat-tab", key), |ui| {
                tab_strip::tab(
                    ui,
                    tab_strip::Tab {
                        title,
                        active,
                        focused: true,
                        preview: false,
                        busy: chat.busy,
                    },
                    |ui, rect, _| agent_logos::paint(ui, &logo, rect),
                )
            })
            .inner;
        if active && reveal {
            item.response.scroll_to_me(Some(egui::Align::Center));
        }
        if item.close_requested {
            Some(TabAction::Close(key))
        } else if item.response.clicked() {
            Some(TabAction::Select(key))
        } else {
            None
        }
    }

    fn chat_tab_content(&self, key: u64) -> Option<(&Chat, Cow<'_, str>)> {
        if key == self.chat_tabs.active {
            return Some((&self.chat, Cow::Borrowed(agent_picker::id(&self.settings))));
        }
        let tab = self.chat_tabs.parked.iter().find(|tab| tab.key == key)?;
        let logo = if tab.choice.id.is_empty() {
            Cow::Owned(
                agent_options::short_agent(&tab.choice.command, &tab.choice.args)
                    .to_ascii_lowercase(),
            )
        } else {
            Cow::Borrowed(tab.choice.id.as_str())
        };
        Some((&tab.chat, logo))
    }
}
