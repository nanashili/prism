use super::parts;
use crate::model::{Project, View};
use crate::ui::{app::Prism, icons::Icon};
use eframe::egui;

struct WorkItem {
    address: String,
    label: String,
    offset: Option<u64>,
}

impl Prism {
    pub(super) fn saved_work(&mut self, ui: &mut egui::Ui) {
        let Some(project) = &self.workspace.project else {
            return;
        };
        let original = project.imported.get("project.json");
        let counts = [
            ("renames", "Renames", Icon::Rename, project.renamed.len()),
            (
                "bookmarks",
                "Bookmarks",
                Icon::Bookmark,
                project.bookmarks.len(),
            ),
            ("notes", "Notes", Icon::Note, project.notes.len()),
            (
                "comments",
                "Comments",
                Icon::Thread,
                original
                    .and_then(|value| value["comments"].as_object())
                    .map_or(0, |values| values.len()),
            ),
            (
                "patches",
                "Patches",
                Icon::Hex,
                project.patches.len()
                    + original
                        .and_then(|value| value["patches"].as_array())
                        .map_or(0, Vec::len),
            ),
            (
                "groups",
                "Groups",
                Icon::Graph,
                original
                    .and_then(|value| value["groups"].as_array())
                    .map_or(0, Vec::len),
            ),
        ];
        let total: usize = counts.iter().map(|(_, _, _, count)| count).sum();
        if total == 0 || !self.sidebar_heading(ui, "work", "Your work", &parts::number(total)) {
            return;
        }
        for (kind, label, icon, count) in counts {
            if count == 0 {
                continue;
            }
            let open = self.workspace.navigation.work == Some(kind);
            if parts::row(
                ui,
                label,
                &parts::number(count),
                parts::RowStyle {
                    icon: Some(icon),
                    selected: open,
                    ..Default::default()
                },
            )
            .clicked()
            {
                self.workspace.navigation.work = if open { None } else { Some(kind) };
                self.workspace.navigation.work_offset = 0;
            }
            if self.workspace.navigation.work == Some(kind) {
                self.work_items(ui, kind, count);
            }
        }
        ui.add_space(10.0);
    }

    fn work_items(&mut self, ui: &mut egui::Ui, kind: &'static str, count: usize) {
        let Some(project) = &self.workspace.project else {
            return;
        };
        let offset = self.workspace.navigation.work_offset;
        for item in items(project, kind, offset) {
            let response = parts::row(
                ui,
                &item.label,
                address_suffix(&item.address),
                parts::RowStyle {
                    icon: Some(Icon::Function),
                    selected: self.workspace.selected == item.address,
                    depth: 1,
                    ..Default::default()
                },
            );
            if response.on_hover_text(&item.label).clicked() {
                if let Some(offset) = item.offset {
                    self.open_function_view("", &item.label, View::Hex);
                    self.workspace.hex_offset = offset;
                    self.load_view(View::Hex);
                } else {
                    self.open_function_view(&item.address, &item.label, View::Pseudocode);
                }
            }
        }
        if offset > 0 || count > 300 {
            ui.horizontal(|ui| {
                if offset > 0 && ui.small_button("Previous").clicked() {
                    self.workspace.navigation.work_offset = offset.saturating_sub(300);
                }
                if offset + 300 < count && ui.small_button("Next 300").clicked() {
                    self.workspace.navigation.work_offset = offset + 300;
                }
            });
        }
    }
}

fn items(project: &Project, kind: &str, offset: usize) -> Vec<WorkItem> {
    let named = |address: &str, label: &str| WorkItem {
        address: address.into(),
        label: label.into(),
        offset: None,
    };
    let function = |address: &str| {
        named(
            address,
            project
                .renamed
                .get(address)
                .map(String::as_str)
                .unwrap_or(address),
        )
    };
    match kind {
        "renames" => project
            .renamed
            .iter()
            .skip(offset)
            .take(300)
            .map(|(address, name)| named(address, name))
            .collect(),
        "notes" => project
            .notes
            .keys()
            .skip(offset)
            .take(300)
            .map(|address| function(address))
            .collect(),
        "bookmarks" => project
            .bookmarks
            .iter()
            .skip(offset)
            .take(300)
            .map(|address| function(address))
            .collect(),
        "comments" => project
            .imported
            .get("project.json")
            .and_then(|value| value["comments"].as_object())
            .into_iter()
            .flat_map(|values| values.iter())
            .skip(offset)
            .take(300)
            .map(|(address, text)| named(address, text.as_str().unwrap_or(address)))
            .collect(),
        "patches" => project
            .patches
            .iter()
            .map(|(offset, bytes)| WorkItem {
                address: format!("0x{offset:x}"),
                label: format!("0x{offset:x} · {} patched bytes", bytes.len()),
                offset: Some(*offset),
            })
            .chain(original_items(project, "patches"))
            .skip(offset)
            .take(300)
            .collect(),
        "groups" => original_items(project, "groups")
            .skip(offset)
            .take(300)
            .collect(),
        _ => Vec::new(),
    }
}

fn original_items<'a>(project: &'a Project, key: &'a str) -> impl Iterator<Item = WorkItem> + 'a {
    project
        .imported
        .get("project.json")
        .and_then(|value| value[key].as_array())
        .into_iter()
        .flatten()
        .map(move |value| {
            let address = value[if key == "groups" { "fn" } else { "addr" }]
                .as_str()
                .unwrap_or("");
            let label = value[if key == "groups" { "label" } else { "bytes" }]
                .as_str()
                .unwrap_or(address);
            WorkItem {
                address: address.into(),
                label: label.into(),
                offset: None,
            }
        })
}

fn address_suffix(address: &str) -> &str {
    let start = address
        .char_indices()
        .rev()
        .nth(4)
        .map_or(0, |(index, _)| index);
    &address[start..]
}
