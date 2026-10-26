use super::{icons::Icon, theme::Palette};
use crate::ui::presentation::PaletteTheme;
use eframe::egui::{self, Color32, Rect};

pub(super) fn paint(ui: &egui::Ui, id: &str, rect: Rect) {
    let dark = ui.visuals().dark_mode;
    if let Some((uri, bytes)) = shipped(id, PaletteTheme::from(dark)) {
        egui::Image::from_bytes(uri, bytes).paint_at(ui, rect);
    } else if let Some(bytes) = ui
        .ctx()
        .data_mut(|data| data.get_temp::<egui::TextureHandle>(egui::Id::new(("agent-logo", id))))
    {
        egui::Image::new(&bytes)
            .tint(if dark {
                Color32::WHITE
            } else {
                Palette::for_context(ui.ctx()).text
            })
            .paint_at(ui, rect);
    } else {
        Icon::Agent.paint(ui, rect, Palette::for_context(ui.ctx()).muted);
    }
}

pub(super) fn shipped(id: &str, dark: PaletteTheme) -> Option<(&'static str, &'static [u8])> {
    let dark = dark == PaletteTheme::Dark;

    macro_rules! logo {
        ($name:literal) => {
            Some((
                concat!("bytes://agents/", $name, ".svg"),
                include_bytes!(concat!("../../resources/upstream/agents/", $name, ".svg"))
                    .as_slice(),
            ))
        };
        ($light:literal, $dark:literal) => {
            if dark { logo!($dark) } else { logo!($light) }
        };
    }
    match id {
        "amp-acp" => logo!("amp-acp"),
        "antigravity-acp" => logo!("antigravity-acp"),
        "aster" => logo!("aster"),
        "claude" | "claude-acp" => logo!("claude-acp"),
        "codebuddy-code" => logo!("codebuddy-code"),
        "codex" | "codex-acp" => logo!("codex-acp"),
        "cortex-code" => logo!("cortex-code"),
        "cursor" => logo!("cursor", "cursor-dark"),
        "deepagents" => logo!("deepagents"),
        "devin" => logo!("devin", "devin-dark"),
        "gemini" => logo!("gemini"),
        "github-copilot-cli" => logo!("github-copilot-cli", "github-copilot-cli-dark"),
        "glm-acp-agent" => logo!("glm-acp-agent"),
        "grok-build" => logo!("grok-build", "grok-build-dark"),
        "junie" => logo!("junie"),
        "kilo" => logo!("kilo", "kilo-dark"),
        "kimi" => logo!("kimi"),
        "minimax-code" => logo!("minimax-code"),
        "mistral-vibe" => logo!("mistral-vibe"),
        "opencode" => logo!("opencode", "opencode-dark"),
        "poolside" => logo!("poolside"),
        "qoder" => logo!("qoder", "qoder-dark"),
        "qwen-code" => logo!("qwen-code"),
        _ => None,
    }
}
