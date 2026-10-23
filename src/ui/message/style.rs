use crate::settings::{Settings, chat_appearance::ChatAppearance};
use crate::ui::{code_fonts, theme::Palette};
use eframe::egui::{self, Color32, FontFamily, FontId, RichText, TextStyle};

pub(in crate::ui) struct ChatStyle {
    pub appearance: ChatAppearance,
    pub font: FontId,
    pub code_font: FontId,
    pub palette: Palette,
    pub dark: bool,
}

#[derive(Clone, Copy)]
pub(in crate::ui) enum BlockKind {
    Thinking,
    Tool,
}

impl ChatStyle {
    pub fn new(context: &egui::Context, settings: &Settings) -> Self {
        let appearance = settings.conversation.appearance.clone();
        let code_family = code_fonts::family(context, &settings.code_font);
        let family = if appearance.font == "Monospace" {
            code_family.clone()
        } else {
            FontFamily::Proportional
        };
        Self {
            font: FontId::new(appearance.text_size, family),
            code_font: FontId::new(appearance.code_size, code_family),
            appearance,
            palette: Palette::for_context(context),
            dark: context.style().visuals.dark_mode,
        }
    }

    pub fn scope<R>(&self, ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
        ui.scope(|ui| {
            let style = ui.style_mut();
            style.url_in_tooltip = true;
            style.visuals.hyperlink_color = self.palette.accent;
            style.visuals.override_text_color = Some(self.palette.text);
            for text_style in [TextStyle::Body, TextStyle::Button, TextStyle::Small] {
                style.text_styles.insert(text_style, self.font.clone());
            }
            style
                .text_styles
                .insert(TextStyle::Monospace, self.code_font.clone());
            style.text_styles.insert(
                TextStyle::Heading,
                FontId::new(self.font.size * 1.25, self.font.family.clone()),
            );
            ui.spacing_mut().item_spacing.y = 0.0;
            content(ui)
        })
        .inner
    }

    pub fn text(&self, text: impl Into<String>) -> RichText {
        RichText::new(text)
            .font(self.font.clone())
            .line_height(Some(self.appearance.line_height(self.font.size)))
    }

    pub fn block<R>(
        &self,
        ui: &mut egui::Ui,
        kind: BlockKind,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let response = egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: 14,
                right: 0,
                top: 4,
                bottom: 4,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if matches!(kind, BlockKind::Thinking) {
                    ui.visuals_mut().override_text_color = Some(self.palette.muted);
                }
                content(ui)
            });
        let rect = response.response.rect;
        let line = if self.appearance.block_colors {
            self.palette.line.lerp_to_gamma(self.accent(kind), 0.25)
        } else {
            self.palette.line
        };
        ui.painter().vline(
            rect.left() + 1.0,
            rect.top() + 2.0..=rect.bottom() - 2.0,
            egui::Stroke::new(2.0, line),
        );
        response.inner
    }

    pub fn accent(&self, kind: BlockKind) -> Color32 {
        if !self.appearance.block_colors {
            return self.palette.text3;
        }
        match (kind, self.dark) {
            (BlockKind::Thinking, true) => Color32::from_rgb(185, 160, 226),
            (BlockKind::Tool, true) => Color32::from_rgb(115, 194, 174),
            (BlockKind::Thinking, false) => Color32::from_rgb(117, 77, 167),
            (BlockKind::Tool, false) => Color32::from_rgb(24, 119, 98),
        }
    }
}
