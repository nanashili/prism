use super::Modifier;
use eframe::egui::Key;

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Key(Key),
    Modifier(Modifier),
    Inactive,
    Arrows,
}

#[derive(Clone, Copy)]
pub(super) struct Keycap {
    pub label: &'static str,
    pub width: f32,
    pub kind: Kind,
}

const fn key(label: &'static str, key: Key, width: f32) -> Keycap {
    Keycap {
        label,
        width,
        kind: Kind::Key(key),
    }
}

const fn modifier(label: &'static str, modifier: Modifier, width: f32) -> Keycap {
    Keycap {
        label,
        width,
        kind: Kind::Modifier(modifier),
    }
}

const COMMON_ROWS: &[&[Keycap]] = &[
    &[
        key("esc", Key::Escape, 1.5),
        key("F1", Key::F1, 1.125),
        key("F2", Key::F2, 1.125),
        key("F3", Key::F3, 1.125),
        key("F4", Key::F4, 1.125),
        key("F5", Key::F5, 1.125),
        key("F6", Key::F6, 1.125),
        key("F7", Key::F7, 1.125),
        key("F8", Key::F8, 1.125),
        key("F9", Key::F9, 1.125),
        key("F10", Key::F10, 1.125),
        key("F11", Key::F11, 1.125),
        key("F12", Key::F12, 1.125),
    ],
    &[
        key("`", Key::Backtick, 1.0),
        key("1", Key::Num1, 1.0),
        key("2", Key::Num2, 1.0),
        key("3", Key::Num3, 1.0),
        key("4", Key::Num4, 1.0),
        key("5", Key::Num5, 1.0),
        key("6", Key::Num6, 1.0),
        key("7", Key::Num7, 1.0),
        key("8", Key::Num8, 1.0),
        key("9", Key::Num9, 1.0),
        key("0", Key::Num0, 1.0),
        key("-", Key::Minus, 1.0),
        key("=", Key::Equals, 1.0),
        key("delete", Key::Backspace, 2.0),
    ],
    &[
        key("tab", Key::Tab, 1.5),
        key("Q", Key::Q, 1.0),
        key("W", Key::W, 1.0),
        key("E", Key::E, 1.0),
        key("R", Key::R, 1.0),
        key("T", Key::T, 1.0),
        key("Y", Key::Y, 1.0),
        key("U", Key::U, 1.0),
        key("I", Key::I, 1.0),
        key("O", Key::O, 1.0),
        key("P", Key::P, 1.0),
        key("[", Key::OpenBracket, 1.0),
        key("]", Key::CloseBracket, 1.0),
        key("\\", Key::Backslash, 1.5),
    ],
    &[
        Keycap {
            label: "caps lock",
            width: 1.75,
            kind: Kind::Inactive,
        },
        key("A", Key::A, 1.0),
        key("S", Key::S, 1.0),
        key("D", Key::D, 1.0),
        key("F", Key::F, 1.0),
        key("G", Key::G, 1.0),
        key("H", Key::H, 1.0),
        key("J", Key::J, 1.0),
        key("K", Key::K, 1.0),
        key("L", Key::L, 1.0),
        key(";", Key::Semicolon, 1.0),
        key("'", Key::Quote, 1.0),
        key("return", Key::Enter, 2.25),
    ],
    &[
        modifier("shift", Modifier::Shift, 2.25),
        key("Z", Key::Z, 1.0),
        key("X", Key::X, 1.0),
        key("C", Key::C, 1.0),
        key("V", Key::V, 1.0),
        key("B", Key::B, 1.0),
        key("N", Key::N, 1.0),
        key("M", Key::M, 1.0),
        key(",", Key::Comma, 1.0),
        key(".", Key::Period, 1.0),
        key("/", Key::Slash, 1.0),
        modifier("shift", Modifier::Shift, 2.75),
    ],
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Platform {
    Mac,
    Windows,
    Linux,
}

impl Platform {
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

const fn inactive(label: &'static str, width: f32) -> Keycap {
    Keycap {
        label,
        width,
        kind: Kind::Inactive,
    }
}

const MAC_FUNCTIONS: &[Keycap] = &[
    key("esc", Key::Escape, 1.5),
    key("F1", Key::F1, 1.0),
    key("F2", Key::F2, 1.0),
    key("F3", Key::F3, 1.0),
    key("F4", Key::F4, 1.0),
    key("F5", Key::F5, 1.0),
    key("F6", Key::F6, 1.0),
    key("F7", Key::F7, 1.0),
    key("F8", Key::F8, 1.0),
    key("F9", Key::F9, 1.0),
    key("F10", Key::F10, 1.0),
    key("F11", Key::F11, 1.0),
    key("F12", Key::F12, 1.0),
    inactive("Touch ID", 1.5),
];

const MAC_MODIFIERS: &[Keycap] = &[
    inactive("fn", 1.0),
    modifier("control", Modifier::Control, 1.0),
    modifier("option", Modifier::Alt, 1.0),
    modifier("command", Modifier::Command, 1.25),
    key("space", Key::Space, 5.75),
    modifier("command", Modifier::Command, 1.25),
    modifier("option", Modifier::Alt, 1.0),
    Keycap {
        label: "arrows",
        width: 2.75,
        kind: Kind::Arrows,
    },
];

const WINDOWS_MODIFIERS: &[Keycap] = &[
    modifier("ctrl", Modifier::Command, 1.25),
    inactive("win", 1.0),
    modifier("alt", Modifier::Alt, 1.25),
    key("space", Key::Space, 5.25),
    modifier("alt", Modifier::Alt, 1.25),
    inactive("fn", 1.0),
    modifier("ctrl", Modifier::Command, 1.25),
    Keycap {
        label: "arrows",
        width: 2.75,
        kind: Kind::Arrows,
    },
];

const LINUX_MODIFIERS: &[Keycap] = &[
    modifier("ctrl", Modifier::Command, 1.25),
    inactive("super", 1.0),
    modifier("alt", Modifier::Alt, 1.25),
    key("space", Key::Space, 5.25),
    modifier("alt", Modifier::Alt, 1.25),
    inactive("fn", 1.0),
    modifier("ctrl", Modifier::Command, 1.25),
    Keycap {
        label: "arrows",
        width: 2.75,
        kind: Kind::Arrows,
    },
];

pub(super) fn rows(platform: Platform) -> [&'static [Keycap]; 6] {
    let (functions, modifiers) = match platform {
        Platform::Mac => (MAC_FUNCTIONS, MAC_MODIFIERS),
        Platform::Windows => (COMMON_ROWS[0], WINDOWS_MODIFIERS),
        Platform::Linux => (COMMON_ROWS[0], LINUX_MODIFIERS),
    };
    [
        functions,
        COMMON_ROWS[1],
        COMMON_ROWS[2],
        COMMON_ROWS[3],
        COMMON_ROWS[4],
        modifiers,
    ]
}

pub(super) fn legend(cap: Keycap, platform: Platform) -> (&'static str, &'static str) {
    if platform != Platform::Mac {
        return match cap.kind {
            Kind::Key(Key::Backspace) => ("backspace", ""),
            Kind::Key(Key::Enter) => ("enter", ""),
            _ => (cap.label, ""),
        };
    }
    match cap.kind {
        Kind::Modifier(Modifier::Control) => ("⌃", "control"),
        Kind::Modifier(Modifier::Alt) => ("⌥", "option"),
        Kind::Modifier(Modifier::Command) => ("⌘", "command"),
        Kind::Modifier(Modifier::Shift) => ("⇧", "shift"),
        Kind::Key(Key::Backspace) => ("⌫", "delete"),
        Kind::Key(Key::Enter) => ("↩", "return"),
        Kind::Key(Key::Tab) => ("⇥", "tab"),
        Kind::Inactive if cap.label == "caps lock" => ("⇪", "caps lock"),
        Kind::Inactive if cap.label == "fn" => ("", "fn"),
        _ => (cap.label, ""),
    }
}
