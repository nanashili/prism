use crate::settings::research::Category;
use crate::ui::{icons::Icon, theme::Palette};
use eframe::egui::{self, FontId, Rect, Stroke};

struct Starter {
    title: &'static str,
    detail: &'static str,
    prompt: &'static str,
    icon: Icon,
}

pub(super) fn show(
    ui: &mut egui::Ui,
    categories: impl Iterator<Item = Category>,
) -> Option<&'static str> {
    let mut starters: Vec<_> = categories.take(2).map(for_category).collect();
    if starters.len() < 2 {
        starters.push(Starter {
            title: "Learn the basics",
            detail: "A guided first investigation",
            prompt: "I'm new to reverse engineering. Explain the basics, then help me choose a small first investigation in Prism. Ask what I have available before suggesting a file to open.",
            icon: Icon::Info,
        });
    }

    let columns = if ui.available_width() >= 520.0 { 2 } else { 1 };
    let width = (ui.available_width() - 10.0 * (columns - 1) as f32) / columns as f32;
    let mut picked = None;
    egui::Grid::new("setup-starters")
        .num_columns(columns)
        .spacing([10.0, 10.0])
        .show(ui, |ui| {
            for (index, starter) in starters.iter().enumerate() {
                if card(ui, starter, width).clicked() {
                    picked = Some(starter.prompt);
                }
                if (index + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
    picked
}

fn for_category(category: Category) -> Starter {
    let (title, detail, prompt, icon) = match category {
        Category::Binaries => (
            "Understand a binary",
            "Find a starting point in native code",
            "Guide me through a first look at a binary in Prism. Help me choose a file, identify its format and find its entry point.",
            Icon::Binary,
        ),
        Category::Firmware => (
            "Explore firmware",
            "Identify an image and its sections",
            "Walk me through inspecting a firmware image in Prism. Start by asking what image or device I have, then help identify its format and sections.",
            Icon::Binary,
        ),
        Category::Web => (
            "Trace a web app",
            "Follow a request through an app",
            "Help me plan a first investigation of a web application. Ask which app or source files I have and what behavior I want to understand.",
            Icon::App,
        ),
        Category::Mobile => (
            "Explore a mobile app",
            "Understand how an app starts",
            "Guide me through a first look at a mobile app in Prism. Ask whether I have an Android or iOS app, then help me choose the right file and find where it starts.",
            Icon::App,
        ),
        Category::Network => (
            "Understand a protocol",
            "Make sense of network traffic",
            "Help me start investigating a network protocol. Ask what application, documentation or traffic capture I have and help me plan the first steps.",
            Icon::Graph,
        ),
        Category::GameEngines => (
            "Explore a game engine",
            "Identify an engine and its assets",
            "Guide me through identifying a game's engine and asset formats. Ask what game files I have available and help me choose where to start in Prism.",
            Icon::App,
        ),
        Category::GameRuntime => (
            "Explore game memory",
            "Learn about runtime data",
            "Help me plan a first investigation of a game's runtime data structures. Ask what game or sample I can inspect and explain how to get started in Prism.",
            Icon::Graph,
        ),
        Category::GameSecurity => (
            "Review game security",
            "Plan a security investigation",
            "Help me plan a review of a game's security architecture. Ask what project I am researching and what behavior I want to understand before suggesting where to start.",
            Icon::ShieldAlert,
        ),
        Category::Graphics => (
            "Explore a shader or model",
            "Understand the rendering or model flow",
            "Guide me through a first look at a shader or machine learning model in Prism. Ask what files I have, help me choose one and explain what to inspect first.",
            Icon::Gpu,
        ),
    };
    Starter {
        title,
        detail,
        prompt,
        icon,
    }
}

fn card(ui: &mut egui::Ui, starter: &Starter, width: f32) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let response = ui.add_sized(
        [width, 76.0],
        egui::Button::new("")
            .fill(p.well)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(12),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            12.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    let content = response.rect.shrink(14.0);
    let icon = Rect::from_center_size(
        content.left_center() + egui::vec2(9.0, 0.0),
        egui::vec2(18.0, 18.0),
    );
    starter.icon.paint(ui, icon, p.muted);
    let text = Rect::from_min_max(egui::pos2(icon.right() + 12.0, content.top()), content.max);
    let opacity = if response.hovered() { 0.72 } else { 1.0 };
    let title = ui.painter().layout_no_wrap(
        starter.title.into(),
        FontId::proportional(13.0),
        p.text.gamma_multiply(opacity),
    );
    let detail = ui.painter().layout(
        starter.detail.into(),
        FontId::proportional(11.5),
        p.muted.gamma_multiply(opacity),
        text.width(),
    );
    let top = text.center().y - (title.size().y + 4.0 + detail.size().y) * 0.5;
    let painter = ui.painter().with_clip_rect(text.intersect(ui.clip_rect()));
    painter.galley(
        egui::pos2(text.left(), top + title.size().y + 4.0),
        detail,
        p.muted,
    );
    painter.galley(egui::pos2(text.left(), top), title, p.text);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), starter.title)
    });
    response.on_hover_text("Use this question as an editable draft")
}
