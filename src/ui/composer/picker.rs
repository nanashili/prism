use super::completion::Suggestion;
use eframe::egui;

#[derive(Default)]
pub(super) struct Keys {
    pub submit: bool,
    pub close: bool,
    pub moved: bool,
}

pub(super) fn keys(ui: &egui::Ui, focused: bool, count: usize, selected: &mut usize) -> Keys {
    if !focused {
        return Keys::default();
    }
    ui.input_mut(|input| {
        let close = input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        let down = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
        let up = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
        let enter = input.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
        let tab = count > 0 && input.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
        if count > 0 {
            if down {
                *selected = (*selected + 1) % count;
            }
            if up {
                *selected = (*selected + count - 1) % count;
            }
        }
        Keys {
            submit: (enter || tab) && count > 0,
            close,
            moved: up || down,
        }
    })
}

pub(super) fn rows(
    ui: &mut egui::Ui,
    entries: &[Suggestion],
    selected: usize,
    marker: char,
    keys: &Keys,
) -> Option<Suggestion> {
    let mut picked = None;
    egui::ScrollArea::vertical()
        .max_height(254.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (index, item) in entries.iter().enumerate() {
                let selected = index == selected;
                let row = item.row(ui, selected);
                if keys.moved && selected {
                    row.scroll_to_me(None);
                }
                if row.clicked() || (keys.submit && selected) {
                    picked = Some(item.clone());
                }
            }
            if entries.is_empty() {
                ui.weak(if marker == '@' {
                    "No matching functions, bookmarks or notes in this project."
                } else {
                    "No matching skills or agent commands."
                });
            }
        });
    picked
}
