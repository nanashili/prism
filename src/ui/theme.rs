use eframe::egui::{self, Color32, FontFamily, FontId, TextStyle};

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub window: Color32,
    pub sidebar: Color32,
    pub card: Color32,
    pub well: Color32,
    pub fill: Color32,
    pub fill2: Color32,
    pub sel: Color32,
    pub deep: Color32,
    pub line: Color32,
    pub text: Color32,
    pub text3: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub raised: Color32,
    pub line3: Color32,
}

impl Palette {
    pub fn for_context(context: &egui::Context) -> Self {
        Self::for_dark(context.style().visuals.dark_mode)
    }

    fn for_dark(dark: bool) -> Self {
        if dark {
            Self {
                window: Color32::from_gray(23),
                sidebar: Color32::from_gray(28),
                card: Color32::from_gray(15),
                well: Color32::from_gray(20),
                fill: Color32::from_gray(31),
                fill2: Color32::from_gray(34),
                sel: Color32::from_gray(38),
                deep: Color32::from_gray(11),
                line: Color32::from_gray(42),
                text: Color32::from_gray(237),
                text3: Color32::from_gray(200),
                muted: Color32::from_gray(143),
                faint: Color32::from_gray(107),
                accent: Color32::from_rgb(115, 173, 233),
                raised: Color32::from_gray(28),
                line3: Color32::from_gray(58),
            }
        } else {
            Self {
                window: Color32::from_rgb(243, 243, 241),
                sidebar: Color32::from_rgb(230, 230, 228),
                card: Color32::WHITE,
                well: Color32::from_rgb(247, 247, 246),
                fill: Color32::from_rgb(236, 236, 234),
                fill2: Color32::from_rgb(233, 233, 231),
                sel: Color32::from_rgb(230, 230, 228),
                deep: Color32::from_rgb(246, 246, 245),
                line: Color32::from_rgb(222, 222, 219),
                text: Color32::from_gray(23),
                text3: Color32::from_gray(58),
                muted: Color32::from_gray(107),
                faint: Color32::from_gray(143),
                accent: Color32::from_rgb(47, 111, 208),
                raised: Color32::from_rgb(241, 241, 239),
                line3: Color32::from_rgb(203, 203, 200),
            }
        }
    }
}

pub(super) fn apply(context: &egui::Context, settings: &crate::settings::Settings) {
    let appearance = settings.appearance.as_str();
    let native_theme = match appearance {
        "Dark" => egui::SystemTheme::Dark,
        "Light" => egui::SystemTheme::Light,
        _ => egui::SystemTheme::SystemDefault,
    };
    context.send_viewport_cmd(egui::ViewportCommand::SetTheme(native_theme));
    match appearance {
        "Dark" => context.set_theme(egui::Theme::Dark),
        "Light" => context.set_theme(egui::Theme::Light),
        _ => context.set_theme(egui::ThemePreference::System),
    }
    context.set_zoom_factor(settings.ui_scale);
    let code_font = super::code::font(context, settings);
    context.all_styles_mut(|style| {
        let palette = Palette::for_dark(style.visuals.dark_mode);
        style
            .text_styles
            .insert(TextStyle::Monospace, code_font.clone());
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(27.0, FontFamily::Proportional),
        );
        style.spacing.item_spacing = egui::vec2(8.0, 0.0);
        style.spacing.button_padding = egui::vec2(9.0, 6.0);
        style.visuals.panel_fill = palette.card;
        style.visuals.window_fill = palette.raised;
        style.visuals.window_stroke = egui::Stroke::new(1.0, palette.line);
        style.visuals.menu_corner_radius = 10.into();
        style.visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 16],
            blur: 40,
            spread: 0,
            color: Color32::from_black_alpha(89),
        };
        style.spacing.menu_margin = egui::Margin::same(6);
        style.visuals.extreme_bg_color = palette.well;
        style.visuals.faint_bg_color = palette.well;
        style.visuals.override_text_color = Some(palette.text);
        style.visuals.selection.bg_fill = palette.fill;
        style.visuals.selection.stroke.color = palette.accent;
        style.visuals.widgets.noninteractive.bg_stroke.color = palette.line;
        style.visuals.widgets.inactive.weak_bg_fill = palette.fill;
        style.visuals.widgets.inactive.bg_stroke.color = palette.line;
    });
}

pub(super) fn fonts(context: &egui::Context) {
    let mut definitions = egui::FontDefinitions::default();
    definitions.font_data.insert(
        "JetBrains Mono".into(),
        egui::FontData::from_static(include_bytes!(
            "../../resources/upstream/fonts/JetBrainsMono-Regular.ttf"
        ))
        .into(),
    );
    definitions
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "JetBrains Mono".into());
    definitions.families.insert(
        FontFamily::Name("JetBrains Mono".into()),
        vec!["JetBrains Mono".into()],
    );
    super::code_fonts::load(context, &mut definitions);
    let paths = [
        "/System/Library/Fonts/SFNS.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            definitions
                .font_data
                .insert("System UI".into(), egui::FontData::from_owned(bytes).into());
            definitions
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "System UI".into());
            break;
        }
    }
    context.set_fonts(definitions);
}
