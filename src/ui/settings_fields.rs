use eframe::egui::{self, RichText};

fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(RichText::new(label).small().weak());
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
}

pub(super) fn agent_settings(ui: &mut egui::Ui, settings: &mut crate::settings::Settings) {
    let before = (settings.agent.clone(), settings.agent_args.clone());
    ui.label(RichText::new("Agent").strong());
    field(ui, "ACP executable", &mut settings.agent);
    field(
        ui,
        "Arguments (quoted arguments supported)",
        &mut settings.agent_args,
    );
    if before != (settings.agent.clone(), settings.agent_args.clone()) {
        settings.agent_id.clear();
        settings.agent_name.clear();
    }
    ui.label(
        RichText::new("Use an ACP adapter such as codex-acp. The Codex preset uses npx if no local adapter is found. Sign in through the agent's own CLI.")
            .small()
            .weak(),
    );
    ui.add_space(16.0);
}

pub(super) fn theme_cards(ui: &mut egui::Ui, appearance: &mut String) {
    let p = super::theme::Palette::for_context(ui.ctx());
    let width = (ui.available_width() - 24.0) / 3.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        for (name, label) in [
            ("Dark", "Dark"),
            ("Light", "Light"),
            ("System", "Match this computer"),
        ] {
            let selected = appearance == name;
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, 130.0), egui::Sense::click());
            ui.painter().rect(
                rect,
                14.0,
                if selected {
                    p.fill
                } else {
                    egui::Color32::TRANSPARENT
                },
                egui::Stroke::new(
                    1.5,
                    if selected {
                        egui::Color32::TRANSPARENT
                    } else {
                        p.line
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let preview = egui::Rect::from_min_size(
                rect.min + egui::vec2(8.0, 8.0),
                egui::vec2(width - 16.0, 84.0),
            );
            theme_preview(ui, preview, name);
            ui.painter().text(
                rect.min + egui::vec2(12.0, 111.0),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                p.text,
            );
            if response.clicked() {
                *appearance = name.into();
            }
        }
    });
}

fn theme_preview(ui: &egui::Ui, rect: egui::Rect, name: &str) {
    use egui::{Color32, Rect, vec2};
    let light = name == "Light";
    let window = if light {
        Color32::from_rgb(243, 243, 241)
    } else {
        Color32::from_gray(23)
    };
    let card = if light {
        Color32::WHITE
    } else {
        Color32::from_gray(15)
    };
    let line = if name == "System" {
        Color32::from_gray(143)
    } else if light {
        Color32::from_gray(212)
    } else {
        Color32::from_gray(58)
    };
    ui.painter().rect_filled(rect, 9.0, window);
    if name == "System" {
        ui.painter()
            .with_clip_rect(Rect::from_min_max(rect.center_top(), rect.max))
            .rect_filled(rect, 9.0, Color32::from_rgb(243, 243, 241));
    }
    let content = Rect::from_min_max(
        rect.min + vec2(rect.width() * 0.28 + 14.0, 8.0),
        rect.max - vec2(8.0, 8.0),
    );
    ui.painter().rect_filled(content, 6.0, card);
    if name == "System" {
        ui.painter()
            .with_clip_rect(Rect::from_min_max(content.center_top(), content.max))
            .rect_filled(content, 6.0, Color32::WHITE);
    }
    for (index, fraction) in [0.8, 0.6, 0.7].into_iter().enumerate() {
        let position = rect.min + vec2(8.0, 12.0 + index as f32 * 10.0);
        ui.painter().rect_filled(
            Rect::from_min_size(position, vec2(rect.width() * 0.28 * fraction, 5.0)),
            3.0,
            line,
        );
    }
    for (index, (fraction, color)) in [
        (0.5, Color32::from_rgb(198, 120, 221)),
        (0.75, line),
        (0.4, Color32::from_rgb(152, 195, 121)),
    ]
    .into_iter()
    .enumerate()
    {
        let position = content.min + vec2(8.0, 8.0 + index as f32 * 10.0);
        ui.painter().rect_filled(
            Rect::from_min_size(position, vec2((content.width() - 16.0) * fraction, 5.0)),
            3.0,
            color,
        );
    }
}
