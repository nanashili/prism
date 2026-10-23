use super::{super::Prism, OpenTab};
use crate::events::Event;
use eframe::egui;

impl Prism {
    pub(in crate::ui) fn receive_chat_event(
        &mut self,
        context: &egui::Context,
        key: u64,
        event: Event,
    ) {
        if key == self.chat_tabs.active {
            self.receive_event(context, event);
            return;
        }

        let Some(index) = self.chat_tabs.parked.iter().position(|tab| tab.key == key) else {
            // Dropping an event also releases any permission or navigation response channel.
            return;
        };
        if let Event::Navigate { answer, .. } = event {
            if answer
                .send(Err(
                    "Switch to this chat before changing the workbench".into()
                ))
                .is_err()
            {
                self.status = "The agent closed its window request".into();
            }
            return;
        }

        let mut tab = self.chat_tabs.parked.remove(index);
        self.exchange_chat(&mut tab);
        std::mem::swap(&mut self.chat_tabs.active, &mut tab.key);
        self.receive_event(context, event);
        std::mem::swap(&mut self.chat_tabs.active, &mut tab.key);
        self.exchange_chat(&mut tab);
        self.chat_tabs.parked.insert(index, tab);
    }

    pub(in crate::ui) fn exchange_chat(&mut self, tab: &mut OpenTab) {
        std::mem::swap(&mut self.chat, &mut tab.chat);
        std::mem::swap(&mut self.agent, &mut tab.agent);
        tab.choice.exchange(&mut self.settings);
    }
}
