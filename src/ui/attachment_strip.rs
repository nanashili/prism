use super::{app::Prism, icons::Icon, theme::Palette, widgets};
use eframe::egui::{self, FontId, Rect};

impl Prism {
    pub(super) fn attachment_strip(&mut self, ui: &mut egui::Ui, bounds: Rect) {
        let mut strip = ui.new_child(egui::UiBuilder::new().max_rect(bounds));
        strip.set_clip_rect(bounds.intersect(ui.clip_rect()));
        let mut remove = None;
        egui::ScrollArea::horizontal()
            .id_salt(("draft-attachments", self.chat_tabs.active))
            .max_height(30.0)
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut strip, |ui| {
                ui.horizontal(|ui| {
                    for (index, attachment) in self.chat.attachments.iter().enumerate() {
                        if attachment_chip(ui, attachment) {
                            remove = Some(index);
                        }
                    }
                });
            });
        if let Some(index) = remove {
            self.chat.attachments.remove(index);
            if let Err(error) = self.save_chat_draft() {
                self.report_error(error);
            }
            ui.ctx().request_repaint();
        }
    }
}

fn attachment_chip(ui: &mut egui::Ui, attachment: &crate::model::Attachment) -> bool {
    let p = Palette::for_context(ui.ctx());
    let frame = egui::Frame::NONE
        .fill(p.fill2)
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(6, 0));
    frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let icon = if attachment.file_path().is_some() {
                    Icon::Source
                } else {
                    Icon::External
                };
                icon.show(ui, 14.0, p.muted);
                let mut label = egui::text::LayoutJob::simple_singleline(
                    attachment.name.clone(),
                    FontId::proportional(12.0),
                    p.text3,
                );
                label.wrap.max_width = 168.0;
                label.wrap.max_rows = 1;
                let text = ui.fonts_mut(|fonts| fonts.layout_job(label));
                ui.add(egui::Label::new(text))
                    .on_hover_text(&attachment.uri);
                widgets::icon_button(ui, Icon::Close, "Remove attachment", 26.0).clicked()
            })
            .inner
        })
        .inner
}
