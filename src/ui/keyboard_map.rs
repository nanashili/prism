use super::{key_combo, keyboard_edit::Editor, shortcut_catalog as catalog, theme::Palette};
use crate::settings::Settings;
use crate::ui::presentation::RowPosition;
use eframe::egui::{self, Align2, FontId, Key, Modifiers, Rect, RichText, Sense};

mod icons;
mod layout;

const KEY_UNIT: f32 = 60.0;
const KEY_GAP: f32 = 6.0;
const KEYBOARD_WIDTH: f32 = 15.0 * KEY_UNIT - KEY_GAP;
const FRAME_PADDING: i8 = 12;

pub(super) const PREFERRED_WIDTH: f32 = KEYBOARD_WIDTH + 2.0 * FRAME_PADDING as f32;

#[derive(Clone, Copy)]
struct KeyGeometry {
    rect: Rect,
    scale: f32,
}

#[derive(Clone, Copy)]
enum Modifier {
    Control,
    Alt,
    Shift,
    Command,
}

pub(super) struct State {
    modifiers: Modifiers,
    selected: Option<Key>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            modifiers: Modifiers {
                command: true,
                mac_cmd: cfg!(target_os = "macos"),
                ctrl: !cfg!(target_os = "macos"),
                ..Modifiers::NONE
            },
            selected: None,
        }
    }
}

impl Modifier {
    fn active(self, modifiers: Modifiers) -> bool {
        match self {
            Self::Control => modifiers.ctrl,
            Self::Alt => modifiers.alt,
            Self::Shift => modifiers.shift,
            Self::Command => modifiers.command,
        }
    }

    fn toggle(self, modifiers: &mut Modifiers) {
        let active = !self.active(*modifiers);
        match self {
            Self::Alt => modifiers.alt = active,
            Self::Shift => modifiers.shift = active,
            Self::Control if cfg!(target_os = "macos") => modifiers.ctrl = active,
            Self::Control | Self::Command => {
                modifiers.command = active;
                if cfg!(target_os = "macos") {
                    modifiers.mac_cmd = active;
                } else {
                    modifiers.ctrl = active;
                }
            }
        }
    }
}

struct Binding<'a> {
    id: &'static str,
    label: &'static str,
    default: &'static str,
    combo: &'a str,
    key: Key,
    modifiers: Modifiers,
}

fn bindings(settings: &Settings) -> Vec<Binding<'_>> {
    catalog::COMMANDS
        .iter()
        .filter(|(id, ..)| catalog::supported(id))
        .filter_map(|&(id, label, _, default)| {
            let combo = catalog::combo(settings, id, default);
            let (modifiers, key) = key_combo::parse(combo)?;
            Some(Binding {
                id,
                label,
                default,
                combo,
                key,
                modifiers,
            })
        })
        .collect()
}

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings, editor: &mut Editor) {
    let p = Palette::for_context(ui.ctx());
    let bindings = bindings(settings);
    egui::Frame::NONE
        .fill(p.fill)
        .stroke((1.0, p.line3))
        .corner_radius(14)
        .inner_margin(FRAME_PADDING)
        .show(ui, |ui| {
            keyboard(ui, &bindings, &mut editor.keyboard);
        });
    ui.add_space(10.0);
    ui.label(
        RichText::new("Click a modifier to reveal shortcuts. Select a highlighted key to edit.")
            .size(11.5)
            .color(p.muted),
    );
    selected_shortcuts(ui, settings, editor);
    if let Some(error) = &editor.import_error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn keyboard(ui: &mut egui::Ui, bindings: &[Binding<'_>], state: &mut State) {
    ui.spacing_mut().item_spacing.y = 0.0;
    if ui.available_width() <= 0.0 {
        return;
    }

    let scale = (ui.available_width() / KEYBOARD_WIDTH).clamp(0.0, 1.0);
    let gap = KEY_GAP * scale;
    let unit = KEY_UNIT * scale;
    let width = KEYBOARD_WIDTH * scale;
    let rows = layout::rows(layout::Platform::current());

    for (index, row) in rows.iter().enumerate() {
        let height = match index {
            0 => 44.0,
            5 => 50.0,
            _ => 54.0,
        } * scale;
        let (bounds, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
        let mut left = bounds.left();

        for (column, &keycap) in row.iter().enumerate() {
            let width = keycap.width * unit - gap;
            let rect =
                Rect::from_min_size(egui::pos2(left, bounds.top()), egui::vec2(width, height));
            let geometry = KeyGeometry { rect, scale };
            let id = ui.id().with(("physical-key", index, column));
            match keycap.kind {
                layout::Kind::Arrows => arrows(ui, geometry, id, bindings, state),
                _ => key(ui, geometry, id, keycap, bindings, state),
            }
            left += width + gap;
        }

        if index + 1 < rows.len() {
            ui.add_space(gap);
        }
    }
}

fn key(
    ui: &mut egui::Ui,
    geometry: KeyGeometry,
    id: egui::Id,
    cap: layout::Keycap,
    bindings: &[Binding<'_>],
    state: &mut State,
) {
    let rect = geometry.rect;
    let matches: Vec<_> = bindings
        .iter()
        .filter(|binding| {
            matches!(cap.kind, layout::Kind::Key(key) if key == binding.key)
                && state.modifiers.matches_exact(binding.modifiers)
        })
        .collect();
    let active = match cap.kind {
        layout::Kind::Modifier(modifier) => modifier.active(state.modifiers),
        layout::Kind::Key(key) => state.selected == Some(key),
        _ => false,
    };
    let interactive = match cap.kind {
        layout::Kind::Modifier(_) => true,
        layout::Kind::Key(key) => bindings.iter().any(|binding| binding.key == key),
        _ => false,
    };
    let sense = if interactive {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, id, sense);
    let description = key_description(cap, &matches);
    response.widget_info(|| match cap.kind {
        layout::Kind::Modifier(_) => egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            active,
            &description,
        ),
        _ => egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled() && interactive,
            &description,
        ),
    });
    paint_key(ui, geometry, &response, active, !matches.is_empty());
    paint_labels(ui, geometry, cap, &matches, active);

    if response.clicked() {
        match cap.kind {
            layout::Kind::Modifier(modifier) => modifier.toggle(&mut state.modifiers),
            layout::Kind::Key(key) => state.selected = Some(key),
            _ => {}
        }
    }
    if interactive {
        response.on_hover_text(description);
    }
}

fn paint_key(
    ui: &egui::Ui,
    geometry: KeyGeometry,
    response: &egui::Response,
    active: bool,
    bound: bool,
) {
    let p = Palette::for_context(ui.ctx());
    let KeyGeometry { rect, scale } = geometry;
    let face = rect.shrink2(egui::vec2(0.0, scale));
    let radius = 6.0 * scale;
    let fill = if active || response.hovered() {
        p.sel
    } else if bound {
        p.raised
    } else {
        p.card
    };
    let outline = if active || response.has_focus() {
        p.accent
    } else {
        p.line3
    };
    ui.painter()
        .rect_filled(rect.translate(egui::vec2(0.0, scale)), radius, p.deep);
    ui.painter().rect(
        face,
        radius,
        fill,
        (scale, outline),
        egui::StrokeKind::Inside,
    );
}

fn key_description(cap: layout::Keycap, bindings: &[&Binding<'_>]) -> String {
    if !bindings.is_empty() {
        return bindings
            .iter()
            .map(|binding| format!("{}  {}", key_combo::text(binding.combo), binding.label))
            .collect::<Vec<_>>()
            .join("\n");
    }
    match cap.kind {
        layout::Kind::Modifier(_) => format!("Toggle {} shortcuts", cap.label),
        layout::Kind::Inactive => cap.label.into(),
        _ => format!("{}: no shortcut with these modifiers", cap.label),
    }
}

fn paint_labels(
    ui: &egui::Ui,
    geometry: KeyGeometry,
    cap: layout::Keycap,
    bindings: &[&Binding<'_>],
    active: bool,
) {
    let p = Palette::for_context(ui.ctx());
    let bound = !bindings.is_empty();
    let color = if active || bound { p.text } else { p.text3 };
    paint_legend(ui, geometry, cap, color, bound);
    if geometry.rect.height() < 40.0 * geometry.scale {
        return;
    }

    let (_, caption) = layout::legend(cap, layout::Platform::current());
    let title = if let Some(binding) = bindings.first() {
        let title = short_label(binding.id, binding.label);
        if bindings.len() > 1 {
            format!("{title} +{}", bindings.len() - 1)
        } else {
            title.to_owned()
        }
    } else {
        caption.to_owned()
    };
    paint_caption(ui, geometry, title, if bound { p.accent } else { p.muted });
}

fn paint_legend(
    ui: &egui::Ui,
    geometry: KeyGeometry,
    cap: layout::Keycap,
    color: egui::Color32,
    bound: bool,
) {
    let KeyGeometry { rect, scale } = geometry;
    let platform = layout::Platform::current();
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    if let layout::Kind::Key(key) = cap.kind
        && let Some(icon) = icons::function_key(key, platform)
    {
        painter.text(
            rect.right_top() + egui::vec2(-4.0, 3.0) * scale,
            Align2::RIGHT_TOP,
            cap.label,
            FontId::proportional(7.5 * scale),
            color,
        );
        let top_offset = if bound { 13.0 } else { 23.0 };
        let center = egui::pos2(rect.center().x, rect.top() + top_offset * scale);
        icon.paint(
            ui,
            Rect::from_center_size(center, egui::vec2(17.0, 17.0) * scale),
            color,
        );
        return;
    }

    let (legend, caption) = layout::legend(cap, platform);
    let compact = rect.height() < 40.0 * scale;
    let position = if compact || (rect.height() < 50.0 * scale && !bound) {
        rect.center()
    } else {
        egui::pos2(rect.center().x, rect.top() + 18.0 * scale)
    };
    if let Some(icon) = icons::symbol(cap, platform) {
        icon.paint(
            ui,
            Rect::from_center_size(position, egui::vec2(19.0, 19.0) * scale),
            color,
        );
    } else {
        let font_size = if caption.is_empty() { 13.0 } else { 17.0 };
        painter.text(
            position,
            Align2::CENTER_CENTER,
            legend,
            FontId::proportional(font_size * scale),
            color,
        );
    }
}

fn paint_caption(ui: &egui::Ui, geometry: KeyGeometry, title: String, color: egui::Color32) {
    let KeyGeometry { rect, scale } = geometry;
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let mut job = egui::text::LayoutJob::simple(
        title,
        FontId::proportional(10.5 * scale),
        color,
        (rect.width() - 8.0 * scale).max(1.0),
    );
    job.wrap.max_rows = 1;
    let galley = painter.layout_job(job);
    painter.galley(
        egui::pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.bottom() - 18.0 * scale,
        ),
        galley,
        color,
    );
}

fn short_label<'a>(id: &str, label: &'a str) -> &'a str {
    match id {
        "palette" => "Search",
        "open" => "Open",
        "save" => "Save",
        "thread" => "Chat",
        "settings" => "Settings",
        "undo" => "Undo",
        "xrefs" => "Refs",
        "bookmark" => "Mark",
        "graph" => "Graph view",
        "pseudo" => "Pseudocode",
        "split" => "Split",
        "agent" => "Agent",
        "stop" => "Stop",
        "patch" => "Patch",
        "debug" => "Debug",
        "comment" => "Note",
        _ => label,
    }
}

fn arrows(
    ui: &mut egui::Ui,
    geometry: KeyGeometry,
    id: egui::Id,
    bindings: &[Binding<'_>],
    state: &mut State,
) {
    let KeyGeometry { rect, scale } = geometry;
    let gap = 3.0 * scale;
    let width = (rect.width() - 2.0 * gap) / 3.0;
    let half_height = (rect.height() - gap) * 0.5;
    let keys = [
        (Key::ArrowLeft, "←", 0.0, half_height + gap),
        (Key::ArrowUp, "↑", width + gap, 0.0),
        (Key::ArrowDown, "↓", width + gap, half_height + gap),
        (Key::ArrowRight, "→", (width + gap) * 2.0, half_height + gap),
    ];
    for (code, label, x, y) in keys {
        let bounds =
            Rect::from_min_size(rect.min + egui::vec2(x, y), egui::vec2(width, half_height));
        key(
            ui,
            KeyGeometry {
                rect: bounds,
                scale,
            },
            id.with(code),
            layout::Keycap {
                label,
                width: 1.0,
                kind: layout::Kind::Key(code),
            },
            bindings,
            state,
        );
    }
}

fn selected_shortcuts(ui: &mut egui::Ui, settings: &mut Settings, editor: &mut Editor) {
    let Some(key) = editor.keyboard.selected else {
        return;
    };
    let p = Palette::for_context(ui.ctx());
    let commands: Vec<_> = bindings(settings)
        .into_iter()
        .filter(|binding| binding.key == key)
        .map(|binding| (binding.id, binding.label, binding.default))
        .collect();
    if commands.is_empty() {
        return;
    }
    ui.add_space(12.0);
    ui.label(
        RichText::new(format!("{} shortcuts", key.symbol_or_name()))
            .size(12.0)
            .color(p.muted),
    );
    ui.add_space(8.0);
    egui::Frame::NONE
        .fill(p.raised)
        .corner_radius(10)
        .show(ui, |ui| {
            for (index, command) in commands.into_iter().enumerate() {
                super::keyboard::row(ui, settings, editor, command, RowPosition::from(index == 0));
            }
        });
}

#[cfg(test)]
mod tests;
