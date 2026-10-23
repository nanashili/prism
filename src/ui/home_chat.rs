use super::app::Prism;
use super::{icons::Icon, theme::Palette};
use crate::model::Screen;
use eframe::egui::{self, Frame, Rect, RichText};

impl Prism {
    pub(super) fn home_conversation(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("home-chat-header")
            .exact_height(44.0)
            .frame(Frame::NONE.inner_margin(egui::Margin::symmetric(16, 7)))
            .show_separator_line(false)
            .show_inside(ui, |ui| self.home_chat_header(ui));

        let width = ui.available_width().min(720.0);
        let rect = Rect::from_center_size(
            ui.available_rect_before_wrap().center(),
            egui::vec2(width, ui.available_height()),
        );
        let mut column = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        self.conversation(&mut column);
    }

    fn home_chat_header(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        let bottom = ui.max_rect().bottom() + 7.0;
        ui.painter()
            .hline(ui.max_rect().x_range(), bottom, (1.0, p.fill));
        ui.horizontal_centered(|ui| {
            let agent = super::agent_picker::label(&self.settings);
            let status = format!("· {agent}");
            let title = self.chat_title();
            let button_width = if self.workspace.project.is_some() {
                124.0
            } else {
                0.0
            };
            let status_width = ui
                .painter()
                .layout_no_wrap(status.clone(), egui::FontId::proportional(13.5), p.muted)
                .size()
                .x;
            let mut title_job = egui::text::LayoutJob::simple_singleline(
                title.into(),
                egui::FontId::proportional(13.5),
                p.text,
            );
            title_job.wrap.max_width =
                (ui.available_width() - status_width - button_width - 76.0).max(40.0);
            title_job.wrap.max_rows = 1;
            let text = ui.fonts_mut(|fonts| fonts.layout_job(title_job));
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(text.size().x, 28.0), egui::Sense::hover());
            ui.painter().galley(
                rect.left_center() - egui::vec2(0.0, text.size().y * 0.5),
                text,
                p.text,
            );
            ui.label(RichText::new(status).color(p.muted));
            self.chat_history_button(ui);
            if super::widgets::icon_button(ui, Icon::NewChat, "New chat", 28.0).clicked() {
                self.new_chat();
            }
            if self.workspace.project.is_some() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let response = super::bench_chrome::header_pill(ui, Icon::Expand, "Workbench");
                    if response.clicked() {
                        self.screen = Screen::Workbench;
                    }
                });
            }
        });
    }

    pub(super) fn chat_history_button(&mut self, ui: &mut egui::Ui) {
        self.past_chats_button(ui);
    }

    pub(super) fn chat_title(&self) -> &str {
        if self.chat.title.is_empty() {
            "New chat"
        } else {
            &self.chat.title
        }
    }
}
