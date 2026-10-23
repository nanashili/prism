use super::{key_combo, shortcut_catalog};
use crate::settings::Settings;
use eframe::egui;

#[derive(Default)]
pub(super) struct Editor {
    pub keyboard: super::keyboard_map::State,
    pub recording: Option<String>,
    pub query: String,
    pub conflict: Option<(String, String, String)>,
    pub import_error: Option<String>,
}

impl Editor {
    pub fn capture(&mut self, context: &egui::Context, settings: &mut Settings) -> bool {
        let Some(id) = self.recording.clone() else {
            return false;
        };
        let events = context.input(|input| input.events.clone());
        for event in events {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                repeat: false,
                ..
            } = event
            else {
                continue;
            };
            context.input_mut(|input| {
                input.consume_key(modifiers, key);
            });
            self.recording = None;
            if key == egui::Key::Escape {
                break;
            }
            let combo = key_combo::record(key, modifiers);
            let conflict = shortcut_catalog::COMMANDS
                .iter()
                .find(|(other, _, _, default)| {
                    *other != id
                        && key_combo::parse(shortcut_catalog::combo(settings, other, default))
                            == key_combo::parse(&combo)
                });
            if let Some((other, _, _, _)) = conflict {
                self.conflict = Some((id, combo, (*other).into()));
            } else {
                settings.views.shortcuts.insert(id, combo);
            }
            break;
        }
        true
    }

    pub fn accept_conflict(&mut self, settings: &mut Settings) {
        if let Some((id, combo, other)) = self.conflict.take() {
            settings.views.shortcuts.insert(other, String::new());
            settings.views.shortcuts.insert(id, combo);
        }
    }

    pub fn import(&mut self, settings: &mut Settings) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Prism shortcuts", &["json"])
            .pick_file()
        else {
            return;
        };
        match read_shortcuts(&path) {
            Ok(shortcuts) => {
                settings.views.shortcuts.extend(shortcuts);
                self.import_error = None;
            }
            Err(error) => self.import_error = Some(error.to_string()),
        }
    }
}

fn read_shortcuts(
    path: &std::path::Path,
) -> crate::error::Result<std::collections::BTreeMap<String, String>> {
    if std::fs::metadata(path)?.len() > 1024 * 1024 {
        return Err("The shortcut file exceeds 1 MB".into());
    }
    let shortcuts: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&std::fs::read(path)?)?;
    for (id, combo) in &shortcuts {
        if !shortcut_catalog::COMMANDS
            .iter()
            .any(|(known, ..)| known == id)
        {
            return Err(format!("Unknown shortcut command: {id}").into());
        }
        if !combo.is_empty() && key_combo::parse(combo).is_none() {
            return Err(format!("Invalid shortcut for {id}: {combo}").into());
        }
    }
    Ok(shortcuts)
}
