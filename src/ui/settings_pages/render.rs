use super::model::{Button, Command, Group, Preview};
use crate::ui::presentation::RowPosition;
use crate::ui::{settings_navigation::SettingsPage, theme::Palette};
use eframe::egui::{self, RichText};

pub(in crate::ui) fn groups(
    ui: &mut egui::Ui,
    groups: Vec<Group<'_>>,
    page: SettingsPage,
    query: &str,
    confirming: &mut Option<Command>,
) -> (Option<Command>, Option<SettingsPage>) {
    let p = Palette::for_context(ui.ctx());
    let mut action = None;
    let mut destination = None;
    for (index, mut group) in groups.into_iter().enumerate() {
        group
            .rows
            .retain(|row| crate::ui::settings_search::row_matches(row, group.label, page, query));
        if group.rows.is_empty() {
            continue;
        }
        if !group.label.is_empty() || !query.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new(group.label).size(12.5).color(p.muted));
                if !query.is_empty() && ui.link(page.title()).clicked() {
                    destination = Some(page);
                }
            });
            ui.add_space(8.0);
        }
        let command = ui
            .push_id(index, |ui| card(ui, &mut group, confirming))
            .inner;
        action = command.or(action);
        if query.is_empty() {
            if let Some(preview) = group.preview {
                preview.show(ui);
            }
            if !group.foot.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new(group.foot).size(12.0).color(p.faint));
            }
        }
        ui.add_space(26.0);
    }
    (action, destination)
}

fn card(
    ui: &mut egui::Ui,
    group: &mut Group<'_>,
    confirming: &mut Option<Command>,
) -> Option<Command> {
    let p = Palette::for_context(ui.ctx());
    let mut action = None;
    egui::Frame::NONE
        .fill(p.raised)
        .corner_radius(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let columns = super::row::Columns::for_rows(&group.rows);
            for (index, row) in group.rows.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    if let Some(command) =
                        super::row::show(ui, row, RowPosition::from(index == 0), columns)
                    {
                        if confirmation(&command).is_some() {
                            *confirming = Some(command);
                        } else {
                            action = Some(command);
                        }
                    }
                    if let Some(command) = confirming.as_ref()
                        && row
                            .buttons
                            .iter()
                            .any(|button| same_confirmation(&button.command, command))
                    {
                        let (confirmed, cancelled) = confirm_bar(ui, command);
                        if confirmed {
                            action = confirming.take();
                        } else if cancelled {
                            *confirming = None;
                        }
                    }
                });
            }
        });
    action
}

fn confirmation(command: &Command) -> Option<(&'static str, &'static str)> {
    match command {
        Command::Reset => Some((
            "Reset every setting to its default? Your projects, chats and shortcuts you changed are kept.",
            "Reset settings",
        )),
        Command::ClearChats => Some((
            "Delete every chat in every project? Renames, notes and patches stay. This can’t be undone.",
            "Delete chats",
        )),
        Command::DeleteStoredChat(_) => Some((
            "Permanently delete this chat and its saved activity? This can’t be undone.",
            "Delete chat",
        )),
        Command::ClearCache => Some((
            "Clear the analysis cache? Nothing is lost, but code takes longer to read the next time you open it.",
            "Clear cache",
        )),
        Command::ClearRecordings => {
            Some(("Delete all GPU recordings? This can’t be undone.", "Delete"))
        }
        _ => None,
    }
}

fn same_confirmation(button: &Command, pending: &Command) -> bool {
    match (button, pending) {
        (Command::DeleteStoredChat(button), Command::DeleteStoredChat(pending)) => {
            button == pending
        }
        _ => std::mem::discriminant(button) == std::mem::discriminant(pending),
    }
}

fn confirm_bar(ui: &mut egui::Ui, command: &Command) -> (bool, bool) {
    let Some((text, label)) = confirmation(command) else {
        return (false, false);
    };
    let mut confirmed = false;
    let mut cancelled = false;
    egui::Frame::NONE
        .outer_margin(egui::Margin {
            left: 18,
            right: 18,
            top: 0,
            bottom: 14,
        })
        .inner_margin(egui::Margin::symmetric(12, 10))
        .corner_radius(10)
        .fill(egui::Color32::from_rgba_unmultiplied(224, 108, 117, 24))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Label::new(RichText::new(text).size(12.5)).wrap());
                cancelled =
                    super::row_control::action_button(ui, &Button::new("Cancel", command.clone()))
                        .clicked();
                let mut button = Button::new(label, command.clone());
                button.danger = true;
                button.primary = true;
                confirmed = super::row_control::action_button(ui, &button).clicked();
            });
        });
    (confirmed, cancelled)
}

impl Preview {
    fn show(self, ui: &mut egui::Ui) {
        match self {
            Self::Chat(settings) => super::chat::preview(ui, &settings),
            Self::Address(settings) => {
                ui.add_space(12.0);
                let p = Palette::for_context(ui.ctx());
                egui::Frame::NONE
                    .fill(p.well)
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for (address, name, bytes) in [
                            ("100003f2c", "check_license", "55 48 89 e5"),
                            ("100003f30", "verify", "f3 0f 1e fa"),
                            ("100004088", "print_usage", "48 83 ec 18"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 16.0;
                                ui.label(
                                    RichText::new(settings.views.address(address))
                                        .monospace()
                                        .size(12.0)
                                        .color(p.muted),
                                );
                                ui.label(
                                    RichText::new(name).monospace().size(12.0).color(p.accent),
                                );
                                ui.label(
                                    RichText::new(if settings.views.address_upper {
                                        bytes.to_uppercase()
                                    } else {
                                        bytes.into()
                                    })
                                    .monospace()
                                    .size(12.0)
                                    .color(p.faint),
                                );
                            });
                            ui.add_space(4.0);
                        }
                    });
            }
        }
    }
}
