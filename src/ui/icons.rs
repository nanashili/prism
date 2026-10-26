use eframe::egui::{self, Color32, Rect};
use std::sync::Arc;

macro_rules! icons {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        #[derive(Clone, Copy)]
        pub(super) enum Icon { Approval, $($variant),+ }

        impl Icon {
            fn asset(self) -> (&'static str, &'static str) {
                match self {
                    Self::Approval => (
                        "bytes://prism/approval.svg",
                        include_str!("../../resources/icons/approval.svg"),
                    ),
                    $(Self::$variant => (
                        concat!("bytes://prism/", $name, ".svg"),
                        include_str!(concat!("../../resources/upstream/icons/", $name, ".svg")),
                    )),+,

                }
            }
        }
    };
}

icons! {
    New => "new", NewChat => "new-chat", Folder => "folder",
    Binary => "binary", App => "app", Gpu => "gpu",
    Slash => "slash", Ask => "asks", Send => "send", Stop => "stop",
    History => "history", Thought => "thought",
    Agent => "agent", ChevronUp => "chevron-up", Disclosure => "disclosure",
    Gear => "gear", Light => "light", Dark => "dark", System => "system",
    Back => "back", Forward => "forward", Search => "search", Play => "play",
    Spark => "spark", Split => "split", Close => "close", Function => "function",
    Assembly => "assembly", Pseudocode => "pseudocode", Graph => "graph", Hex => "hex",
    Strings => "strings", Imports => "library", Sort => "sort", Sidebar => "sidebar",
    Segments => "segments", Source => "source",
    Bookmark => "bookmark", Note => "note", Panel => "panel", More => "more",
    Bolt => "bolt", Export => "export-all", Save => "done",
    Copy => "copy", UpDown => "updown", Expand => "expand",
    Check => "check", Retry => "retry", Star => "star", Rename => "rename", Thread => "thread",
    Plan => "plan", ShieldAlert => "shield-alert", Download => "download", External => "external",
    Text => "text", ZoomOut => "zoom-out", ZoomIn => "zoom-in",
    ExportPseudo => "export-pseudo", ExportAsm => "export-asm", ExportObjc => "export-objc",
    ExportPatch => "export-patch", ExportFolder => "export-folder",
    References => "references", Info => "info", Trash => "trash",
}

impl Icon {
    pub fn paint(self, ui: &egui::Ui, rect: Rect, color: Color32) {
        let (uri, source) = self.asset();
        let bytes = ui.ctx().data_mut(|data| {
            data.get_temp_mut_or_insert_with(egui::Id::new(uri), || {
                Arc::<[u8]>::from(source.replace("\"black\"", "\"white\"").into_bytes())
            })
            .clone()
        });
        egui::Image::from_bytes(uri, bytes)
            .tint(color)
            .paint_at(ui, rect);
    }

    pub fn show(self, ui: &mut egui::Ui, size: f32, color: Color32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        self.paint(ui, rect, color);
    }
}
