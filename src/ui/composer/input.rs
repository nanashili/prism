use super::{completion::Completion, input_id};
use crate::ui::presentation::InputFocus;
use crate::ui::{app::Prism, theme::Palette};
use eframe::egui::{self, FontId, TextEdit};

impl Prism {
    pub(super) fn composer_input(&mut self, ui: &mut egui::Ui, size: f32, height: f32) -> bool {
        let id = input_id(self.chat_tabs.active);
        let mut completion = Completion::load(ui.ctx(), id);
        let was_completing = completion.is_open();
        // Consume picker navigation before TextEdit moves the caret or handles Return.
        let keys = completion.keys(
            ui,
            InputFocus::from(ui.memory(|memory| memory.has_focus(id))),
        );
        let mut input = self.draft_input(ui, id, size, height);
        completion.update(
            &self.chat.draft,
            input.cursor_range,
            InputFocus::from(input.response.has_focus() || input.response.lost_focus()),
        );
        let completing = was_completing || completion.is_open();
        self.composer_suggestions(ui, &mut input, &mut completion, &keys);
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    tab: completion.is_open(),
                    escape: completion.is_open(),
                },
            );
        });
        completion.store(ui.ctx(), id);

        !completing
            && input.response.has_focus()
            && ui.input_mut(|input| {
                input.consume_key(
                    if self.settings.conversation.send_key == "Command Return" {
                        egui::Modifiers::COMMAND
                    } else {
                        egui::Modifiers::NONE
                    },
                    egui::Key::Enter,
                )
            })
    }

    fn draft_input(
        &mut self,
        ui: &mut egui::Ui,
        id: egui::Id,
        size: f32,
        height: f32,
    ) -> egui::text_edit::TextEditOutput {
        let hint = if self.screen == crate::model::Screen::Setup {
            "What would you like to learn or investigate?"
        } else if self.screen == crate::model::Screen::Workbench {
            "Ask about this code, / for skills, @ to mention"
        } else if self.chat.messages.is_empty() {
            "Ask anything, or drop an app, binary, shader or model here"
        } else {
            "Ask a follow-up, / for skills, @ to mention"
        };
        egui::ScrollArea::vertical()
            .id_salt(("composer-draft", self.chat_tabs.active))
            .max_height(height)
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                TextEdit::multiline(&mut self.chat.draft)
                    .id(id)
                    .hint_text(
                        egui::RichText::new(hint).color(Palette::for_context(ui.ctx()).faint),
                    )
                    .font(FontId::proportional(size))
                    .margin(0.0)
                    .frame(false)
                    .desired_width(f32::INFINITY)
                    .min_size(egui::vec2(ui.available_width(), height))
                    .desired_rows(1)
                    .char_limit(32_000)
                    .return_key(egui::KeyboardShortcut::new(
                        if self.settings.conversation.send_key == "Command Return" {
                            egui::Modifiers::NONE
                        } else {
                            egui::Modifiers::SHIFT
                        },
                        egui::Key::Enter,
                    ))
                    .show(ui)
            })
            .inner
    }
}
