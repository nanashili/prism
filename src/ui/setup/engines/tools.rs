use super::{Path, Tool};
use crate::ui::presentation::ToolScope;
use eframe::egui;

pub(super) fn for_workspace(include_mobile: ToolScope) -> Vec<Tool> {
    let include_mobile = include_mobile == ToolScope::WithMobile;

    let mut tools = Vec::new();
    tools.push(Tool {
        name: "REA (Rust)",
        description: "Install or build nanashili/rea-rust, then choose its native executable. The matching worker is discovered beside it.",
        logo: egui::include_image!("../../../../resources/upstream/engines/rea.svg"),
        path: Path::Rea,
        url: "https://github.com/nanashili/rea-rust#build-from-source",
        action_label: "Install / build",
        automatic: true,
    });
    tools.push(Tool {
        name: "Rizin",
        description: "Inspect native binaries and disassemble code.",
        logo: egui::include_image!("../../../../resources/upstream/engines/rizin.svg"),
        path: Path::Rizin,
        url: "https://rizin.re",
        action_label: "Download",
        automatic: false,
    });
    if include_mobile {
        tools.push(Tool {
            name: "jadx",
            description: "Read Android apps as Java source code.",
            logo: egui::include_image!("../../../../resources/upstream/engines/jadx.png"),
            path: Path::Jadx,
            url: "https://github.com/skylot/jadx",
            action_label: "Download",
            automatic: false,
        });
    }
    tools
}
