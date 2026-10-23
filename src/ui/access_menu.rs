use super::agent_option_rows::{self, ChoiceRow};
use super::app::Prism;
use super::{agent_picker, icons::Icon, settings_navigation::SettingsPage, theme::Palette};
use crate::agent::options::{Choice, OptionMenu};
use crate::model::Screen;
use eframe::egui::{self, FontId, Rect, RichText, Sense};

impl Prism {
    pub(super) fn access_options(&mut self, ui: &mut egui::Ui, menu: &OptionMenu) {
        let codex = agent_picker::id(&self.settings) == "codex-acp";
        let heading = if codex {
            "How should Codex actions be approved?"
        } else {
            &menu.name
        };

        ui.spacing_mut().item_spacing.y = 0.0;
        if access_heading(ui, heading).clicked() {
            self.settings_ui.select(SettingsPage::Permissions);
            self.screen = Screen::Settings;
            ui.close();
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt(("access-options", &menu.id))
            .max_height(340.0)
            .show(ui, |ui| {
                for choice in menu.choices.iter().filter(|choice| {
                    !matches!(
                        agent_option_rows::mode_icon(&choice.value, &choice.name),
                        Icon::Plan
                    )
                }) {
                    let row = presentation(choice, codex);
                    if row.show(ui, choice.value == menu.current).clicked() {
                        self.pick_option(menu, choice.value.clone());
                        ui.close();
                    }
                }
            });
    }
}

fn access_heading(ui: &mut egui::Ui, heading: &str) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let title = ui.painter().layout(
        heading.into(),
        FontId::proportional(11.5),
        p.muted,
        (ui.available_width() - 92.0).max(80.0),
    );
    let height = (title.size().y + 12.0).max(26.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    ui.painter().galley(
        egui::pos2(rect.left() + 8.0, rect.center().y - title.size().y / 2.0),
        title,
        p.muted,
    );

    let help = Rect::from_min_max(
        egui::pos2(rect.right() - 78.0, rect.top()),
        rect.right_bottom() - egui::vec2(8.0, 0.0),
    );
    ui.put(
        help,
        egui::Label::new(
            RichText::new("Learn more")
                .size(11.5)
                .color(p.muted)
                .underline(),
        )
        .selectable(false)
        .sense(Sense::click()),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
    .on_hover_text("Open permission settings")
}

fn presentation(choice: &Choice, codex: bool) -> ChoiceRow<'_> {
    let label = if codex {
        match choice.value.as_str() {
            "read-only" => "Ask for approval",
            "agent" => "Approve for me",
            _ => &choice.name,
        }
    } else {
        &choice.name
    };
    let icon = if codex {
        match choice.value.as_str() {
            "read-only" => Icon::Approval,
            "agent" => Icon::Ask,
            _ => agent_option_rows::mode_icon(&choice.value, &choice.name),
        }
    } else {
        agent_option_rows::mode_icon(&choice.value, &choice.name)
    };

    ChoiceRow {
        label,
        description: &choice.description,
        icon: Some(icon),
        warning: matches!(icon, Icon::ShieldAlert),
    }
}
