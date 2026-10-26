use crate::ui::presentation::TypingState;
use eframe::egui::{self, Key, Modifiers};

pub(super) fn parse(combo: &str) -> Option<(Modifiers, Key)> {
    let mut modifiers = Modifiers::NONE;
    let mut rest = combo;
    while let Some((head, tail)) = rest.split_once('-') {
        match head {
            "ctrl" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "cmd" => {
                modifiers.command = true;
                if cfg!(target_os = "macos") {
                    modifiers.mac_cmd = true;
                } else {
                    modifiers.ctrl = true;
                }
            }
            _ => break,
        }
        rest = tail;
    }
    let key = match rest {
        "enter" => Key::Enter,
        "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "left" => Key::ArrowLeft,
        "right" => Key::ArrowRight,
        _ => Key::from_name(rest).or_else(|| {
            Key::ALL
                .iter()
                .copied()
                .find(|key| key.name().eq_ignore_ascii_case(rest))
        })?,
    };
    Some((modifiers, key))
}

pub(super) fn record(key: Key, modifiers: Modifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.ctrl && cfg!(target_os = "macos") {
        parts.push("ctrl".into());
    }
    if modifiers.alt {
        parts.push("alt".into());
    }
    if modifiers.shift {
        parts.push("shift".into());
    }
    if modifiers.command || (modifiers.ctrl && !cfg!(target_os = "macos")) {
        parts.push("cmd".into());
    }
    parts.push(key.name().to_lowercase());
    parts.join("-")
}

pub(super) fn text(combo: &str) -> String {
    let caps = caps(combo);
    if caps.is_empty() {
        "Add".into()
    } else {
        caps.join(if cfg!(target_os = "macos") { "" } else { "+" })
    }
}

pub(super) fn caps(combo: &str) -> Vec<String> {
    let Some((modifiers, key)) = parse(combo) else {
        return Vec::new();
    };
    let mut caps = Vec::new();
    if modifiers.ctrl && cfg!(target_os = "macos") {
        caps.push("⌃".into());
    }
    if !cfg!(target_os = "macos") && (modifiers.ctrl || modifiers.command) {
        caps.push("Ctrl".into());
    }
    if modifiers.alt {
        caps.push(
            if cfg!(target_os = "macos") {
                "⌥"
            } else {
                "Alt"
            }
            .into(),
        );
    }
    if modifiers.shift {
        caps.push(
            if cfg!(target_os = "macos") {
                "⇧"
            } else {
                "Shift"
            }
            .into(),
        );
    }
    if modifiers.command && cfg!(target_os = "macos") {
        caps.push("⌘".into());
    }
    caps.push(
        match key {
            Key::Enter if cfg!(target_os = "macos") => "↩",
            Key::Enter => "Enter",
            Key::Backspace if cfg!(target_os = "macos") => "⌫",
            Key::Backspace => "Backspace",
            Key::Escape => "Esc",
            _ => key.symbol_or_name(),
        }
        .into(),
    );
    caps
}

pub(super) fn pressed(context: &egui::Context, combo: &str, typing: TypingState) -> bool {
    let typing = typing == TypingState::Typing;

    let Some((modifiers, key)) = parse(combo) else {
        return false;
    };
    if typing && !modifiers.command && !modifiers.ctrl {
        return false;
    }
    context.input_mut(|input| input.consume_key(modifiers, key))
}
