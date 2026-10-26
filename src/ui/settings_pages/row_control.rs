use super::model::{Button, Command, Control, Row};
use super::row::{select_width, text_width};
use crate::ui::{settings_controls as controls, theme::Palette};
use eframe::egui::{self, RichText, Stroke};

pub(super) fn show(ui: &mut egui::Ui, row: &mut Row<'_>) -> Option<Command> {
    if let Some((text, good)) = &row.status {
        status(ui, text, *good);
    }
    let mut action = None;
    match &mut row.control {
        Control::None => {}
        Control::Toggle(value) => controls::toggle(ui, &row.title, value),
        Control::Choice(value, options, segmented) => {
            if *segmented {
                controls::segmented(ui, value, options);
            } else {
                controls::select_sized(ui, value, options, select_width(ui, value));
            }
        }
        Control::Range(value, min, max, step, unit) => {
            controls::range(ui, &row.title, value, *min..=*max, *step, unit);
        }
        Control::Memory(value) => {
            numeric_choice(ui, *value, &[256, 512, 1024, 2048, 4096, 8192], "MiB");
        }
        Control::Timeout(value) => {
            numeric_choice(ui, *value, &[30, 60, 120, 300, 600, 900, 1800], "seconds");
        }
        Control::Text(value, hint) => {
            controls::text_input(ui, value, hint, 220.0, 2048);
        }
        Control::Permission(tool, level) => {
            let mut selected = level.label().to_owned();
            controls::segmented(ui, &mut selected, &["Allow", "Ask", "Never"]);
            if selected != level.label()
                && let Some(level) = crate::settings::permissions::Level::from_label(&selected)
            {
                action = Some(Command::Permission(*tool, level));
            }
        }
        Control::Skill(id, enabled) => {
            let before = *enabled;
            controls::toggle(ui, &row.title, enabled);
            if *enabled != before {
                action = Some(Command::Skill(id.clone(), *enabled));
            }
        }
    }
    for button in &row.buttons {
        if action_button(ui, button).clicked() {
            action = Some(button.command.clone());
        }
    }
    action
}

fn numeric_choice<T: Copy + PartialEq + std::fmt::Display>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[T],
    unit: &str,
) {
    let mut label = format!("{value} {unit}");
    let labels: Vec<_> = options
        .iter()
        .map(|option| format!("{option} {unit}"))
        .collect();
    let choices: Vec<_> = labels.iter().map(String::as_str).collect();
    controls::select_sized(ui, &mut label, &choices, 150.0);
    if let Some(index) = labels.iter().position(|option| *option == label) {
        *value = options[index];
    }
}

fn status(ui: &mut egui::Ui, text: &str, good: bool) {
    let p = Palette::for_context(ui.ctx());
    let color = if !good {
        p.muted
    } else if ui.visuals().dark_mode {
        egui::Color32::from_rgb(152, 195, 121)
    } else {
        egui::Color32::from_rgb(63, 138, 62)
    };
    egui::Frame::NONE
        .fill(if good {
            color.gamma_multiply(0.14)
        } else {
            p.fill2
        })
        .corner_radius(11)
        .inner_margin(egui::Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).color(color));
        });
}

pub(in crate::ui) fn action_button(ui: &mut egui::Ui, button: &Button) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let danger = egui::Color32::from_rgb(224, 108, 117);
    let color = if button.danger && button.primary {
        egui::Color32::WHITE
    } else if button.danger {
        danger
    } else if button.primary {
        p.card
    } else {
        p.text3
    };
    let response = ui
        .add_enabled_ui(button.unavailable.is_none(), |ui| {
            if !button.primary && !button.danger {
                return controls::button(ui, &button.label);
            }
            ui.add_sized(
                [text_width(ui, &button.label, 12.5) + 22.0, 28.0],
                egui::Button::new(RichText::new(&button.label).size(12.5).color(color))
                    .corner_radius(8)
                    .fill(if button.primary && button.danger {
                        danger
                    } else if button.primary {
                        p.text
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(Stroke::new(if button.primary { 0.0 } else { 1.0 }, p.line3)),
            )
        })
        .inner;
    if let Some(reason) = button.unavailable {
        response.on_disabled_hover_text(reason)
    } else {
        response
    }
}
