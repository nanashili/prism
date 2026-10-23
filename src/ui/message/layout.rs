use super::style::ChatStyle;
use eframe::egui;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) struct LayoutKey {
    width: u32,
    revision: u64,
}

impl LayoutKey {
    pub fn new(ui: &egui::Ui, style: &ChatStyle, thinking: bool) -> Self {
        let mut revision = std::collections::hash_map::DefaultHasher::new();
        style.appearance.font.hash(&mut revision);
        style.appearance.block_colors.hash(&mut revision);
        style.font.hash(&mut revision);
        style.code_font.hash(&mut revision);
        style.dark.hash(&mut revision);
        thinking.hash(&mut revision);
        for value in [
            style.appearance.text_size,
            style.appearance.code_size,
            style.appearance.line_spacing,
            style.appearance.paragraph_spacing,
            style.appearance.message_spacing,
            ui.pixels_per_point(),
            ui.spacing().item_spacing.x,
            ui.spacing().item_spacing.y,
            ui.spacing().button_padding.x,
            ui.spacing().button_padding.y,
            ui.spacing().interact_size.y,
            ui.spacing().indent,
        ] {
            value.to_bits().hash(&mut revision);
        }
        ui.fonts(|fonts| {
            let definitions = fonts.definitions();
            definitions.families.hash(&mut revision);
            for (name, data) in &definitions.font_data {
                name.hash(&mut revision);
                std::sync::Arc::as_ptr(data).hash(&mut revision);
            }
        });
        Self {
            width: ui.available_width().to_bits(),
            revision: revision.finish(),
        }
    }
}
