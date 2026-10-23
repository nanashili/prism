use crate::model::Approval;
use crate::ui::{app::Prism, icons::Icon, theme::Palette};
use eframe::egui::{self, Frame, RichText};

impl Prism {
    pub(super) fn permission_card(&mut self, ui: &mut egui::Ui) {
        if self
            .chat
            .permission
            .as_ref()
            .is_some_and(|permission| !permission.active.load(std::sync::atomic::Ordering::Relaxed))
        {
            self.chat.permission = None;
        }
        let Some(permission) = &self.chat.permission else {
            return;
        };
        let choice = show(ui, &permission.request);
        if let Some(choice) = choice
            && let Some(permission) = self.chat.permission.take()
            && permission.answer.send(choice).is_err()
        {
            self.report_error("The agent stopped before the permission answer arrived");
        }
    }
}

fn show(ui: &mut egui::Ui, request: &Approval) -> Option<Option<String>> {
    let p = Palette::for_context(ui.ctx());
    Frame::NONE
        .fill(p.raised)
        .corner_radius(12)
        .inner_margin(12)
        .show(ui, |ui| {
            let proposal = request.proposal.as_ref();
            let title = proposal.map_or(request.title.as_str(), |offer| &offer.title);
            ui.horizontal_top(|ui| {
                Icon::Ask.show(ui, 15.0, p.accent);
                ui.add(egui::Label::new(RichText::new(title).strong()).truncate())
                    .on_hover_text(title);
            });
            ui.add_space(6.0);
            let reason = proposal.map_or("The agent needs your approval to continue.", |offer| {
                offer.reason.as_str()
            });
            egui::ScrollArea::vertical()
                .id_salt("approval-details")
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(reason).size(12.0).color(p.muted)).wrap(),
                    );
                    if proposal.is_none() && !request.preview.is_empty() {
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&request.preview).monospace().size(11.5),
                            )
                            .selectable(true)
                            .wrap(),
                        );
                    }
                });
            ui.add_space(8.0);
            choices(ui, request)
        })
        .inner
}

fn choices(ui: &mut egui::Ui, request: &Approval) -> Option<Option<String>> {
    let p = Palette::for_context(ui.ctx());
    let mut answer = None;
    ui.horizontal_wrapped(|ui| {
        let first_allow = request.options.iter().position(|option| option.allows);
        for (index, choice) in request.options.iter().enumerate() {
            let label = request
                .proposal
                .as_ref()
                .filter(|_| Some(index) == first_allow)
                .map_or(choice.label.as_str(), |offer| offer.apply);
            let text = RichText::new(label).size(12.5);
            let button = if Some(index) == first_allow {
                egui::Button::new(text.color(p.card)).fill(p.text)
            } else {
                egui::Button::new(text)
            };
            if ui
                .add(button.corner_radius(8))
                .on_hover_text(&choice.label)
                .clicked()
            {
                answer = Some(Some(choice.id.clone()));
            }
        }
        if ui.button("Skip").clicked() {
            answer = Some(
                request
                    .options
                    .iter()
                    .find(|choice| !choice.allows)
                    .map(|choice| choice.id.clone()),
            );
        }
    });
    answer
}
