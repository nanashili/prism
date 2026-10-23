use super::layout::{Keycap, Kind, Platform};
use eframe::egui::{self, Color32, Key, Rect};

macro_rules! icons {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        #[derive(Clone, Copy)]
        pub(super) enum Icon { $($variant),+ }

        impl Icon {
            pub fn paint(self, ui: &egui::Ui, rect: Rect, color: Color32) {
                let source = match self {
                    $(Self::$variant => egui::include_image!(
                        concat!("../../../resources/icons/keyboard/", $name, ".svg")
                    )),+,
                };
                egui::Image::new(source).tint(color).paint_at(ui, rect);
            }
        }
    };
}

icons! {
    BrightnessDown => "brightness-down",
    BrightnessUp => "brightness-up",
    MissionControl => "mission-control",
    Spotlight => "spotlight",
    Dictation => "dictation",
    DoNotDisturb => "do-not-disturb",
    Rewind => "rewind",
    PlayPause => "play-pause",
    FastForward => "fast-forward",
    Mute => "mute",
    VolumeDown => "volume-down",
    VolumeUp => "volume-up",
    Globe => "globe",
    TouchId => "touch-id",
    Windows => "windows",
    Tab => "tab",
}

pub(super) fn function_key(key: Key, platform: Platform) -> Option<Icon> {
    if platform != Platform::Mac {
        return None;
    }
    match key {
        Key::F1 => Some(Icon::BrightnessDown),
        Key::F2 => Some(Icon::BrightnessUp),
        Key::F3 => Some(Icon::MissionControl),
        Key::F4 => Some(Icon::Spotlight),
        Key::F5 => Some(Icon::Dictation),
        Key::F6 => Some(Icon::DoNotDisturb),
        Key::F7 => Some(Icon::Rewind),
        Key::F8 => Some(Icon::PlayPause),
        Key::F9 => Some(Icon::FastForward),
        Key::F10 => Some(Icon::Mute),
        Key::F11 => Some(Icon::VolumeDown),
        Key::F12 => Some(Icon::VolumeUp),
        _ => None,
    }
}

pub(super) fn symbol(cap: Keycap, platform: Platform) -> Option<Icon> {
    match (platform, cap.kind, cap.label) {
        (Platform::Mac, Kind::Inactive, "fn") => Some(Icon::Globe),
        (Platform::Mac, Kind::Inactive, "Touch ID") => Some(Icon::TouchId),
        (Platform::Windows, Kind::Inactive, "win") => Some(Icon::Windows),
        (Platform::Mac, Kind::Key(Key::Tab), _) => Some(Icon::Tab),
        _ => None,
    }
}
