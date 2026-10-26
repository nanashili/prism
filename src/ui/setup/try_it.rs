use crate::settings::research::Category;
use crate::ui::presentation::{Border, Caret, OpenTarget};
use crate::ui::{
    agent_logos, agent_picker, app::Prism, composer, icons::Icon, theme::Palette, widgets,
};
use eframe::egui::{self, FontId, Rect, RichText, Stroke};

mod starters;

impl Prism {
    pub(super) fn setup_try_it(&mut self, ui: &mut egui::Ui) {
        let opening = self.pending > 0;
        let available = !opening && !self.chat_tabs.any_busy(&self.chat);
        if ui
            .add_enabled_ui(available, |ui| open_file(ui, opening))
            .inner
            .clicked()
        {
            self.choose_open(OpenTarget::File);
        }

        ui.add_space(20.0);
        self.setup_question_heading(ui);
        ui.add_space(8.0);
        ui.add_enabled_ui(available, |ui| self.composer(ui));
        ui.add_space(16.0);

        let p = Palette::for_context(ui.ctx());
        ui.label(
            RichText::new("Need an idea? Choose a starting point.")
                .size(12.0)
                .color(p.muted),
        );
        ui.add_space(6.0);
        let categories = Category::ALL
            .into_iter()
            .filter(|category| self.settings.research.contains(*category));
        if let Some(prompt) = starters::show(ui, categories) {
            self.chat.draft = prompt.into();
            self.chat.mentions.clear();
            ui.memory_mut(|memory| memory.request_focus(composer::input_id(self.chat_tabs.active)));
        }
    }

    fn setup_question_heading(&mut self, ui: &mut egui::Ui) {
        let p = Palette::for_context(ui.ctx());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Or start with a question")
                    .size(13.0)
                    .color(p.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::chip_tinted(ui, None, "Change", Border::Hidden, Caret::Hidden, p.muted)
                    .clicked()
                {
                    self.settings_ui.setup_step = 2;
                }
                ui.label(
                    RichText::new(agent_picker::label(&self.settings))
                        .size(12.0)
                        .color(p.muted),
                );
                let (logo, _) =
                    ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                agent_logos::paint(ui, agent_picker::id(&self.settings), logo);
            });
        });
    }

    pub(in crate::ui) fn setup_composer_controls(&mut self, ui: &mut egui::Ui) -> bool {
        let p = Palette::for_context(ui.ctx());
        ui.horizontal_centered(|ui| {
            if widgets::chip_tinted(
                ui,
                Some(Icon::New),
                "Add files",
                Border::Hidden,
                Caret::Hidden,
                p.muted,
            )
            .clicked()
            {
                self.pick_attachments();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = !self.chat.draft.trim().is_empty() || !self.chat.attachments.is_empty();
                let text_color = if ready { p.card } else { p.muted };
                let fill = if ready { p.text } else { p.fill };

                ui.add_enabled(
                    ready,
                    egui::Button::new(RichText::new("Start chat").size(12.0).color(text_color))
                        .fill(fill)
                        .corner_radius(8)
                        .min_size(egui::vec2(96.0, 30.0)),
                )
                .clicked()
            })
            .inner
        })
        .inner
    }
}

fn open_file(ui: &mut egui::Ui, opening: bool) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let response = ui.add_sized(
        [ui.available_width(), 92.0],
        egui::Button::new("")
            .fill(p.well)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(14),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            14.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    let content = response.rect.shrink(20.0);
    let icon = Rect::from_center_size(
        content.left_center() + egui::vec2(16.0, 0.0),
        egui::vec2(28.0, 28.0),
    );
    Icon::Folder.paint(ui, icon, p.muted);
    let text = Rect::from_min_max(
        egui::pos2(icon.right() + 18.0, content.top()),
        content.right_bottom() - egui::vec2(24.0, 0.0),
    );
    let title = if opening {
        "Opening your file…"
    } else {
        "Open your first file"
    };
    let detail = if opening {
        "Your workspace will open when the file is ready."
    } else {
        "Choose an app, binary, shader, model or project. You can also drop it here."
    };
    let opacity = if response.hovered() { 0.72 } else { 1.0 };
    let title = ui.painter().layout_no_wrap(
        title.into(),
        FontId::proportional(15.0),
        p.text.gamma_multiply(opacity),
    );
    let detail = ui.painter().layout(
        detail.into(),
        FontId::proportional(12.0),
        p.muted.gamma_multiply(opacity),
        text.width(),
    );
    let top = text.center().y - (title.size().y + 5.0 + detail.size().y) * 0.5;
    let painter = ui.painter().with_clip_rect(text.intersect(ui.clip_rect()));
    painter.galley(
        egui::pos2(text.left(), top + title.size().y + 5.0),
        detail,
        p.muted,
    );
    painter.galley(egui::pos2(text.left(), top), title, p.text);
    Icon::Forward.paint(
        ui,
        Rect::from_center_size(
            content.right_center() - egui::vec2(6.0, 0.0),
            egui::vec2(14.0, 14.0),
        ),
        p.muted,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            "Open your first file",
        )
    });
    response
}
