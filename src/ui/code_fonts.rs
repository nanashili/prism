use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

pub(super) fn load(context: &egui::Context, definitions: &mut FontDefinitions) {
    let mut available = vec!["JetBrains Mono".to_owned()];
    for &(name, path) in system_fonts() {
        if let Ok(bytes) = std::fs::read(path) {
            definitions
                .font_data
                .insert(name.into(), FontData::from_owned(bytes).into());
            definitions
                .families
                .insert(FontFamily::Name(name.into()), vec![name.into()]);
            available.push(name.into());
        }
    }
    context.data_mut(|data| data.insert_temp(egui::Id::new("code-fonts"), available));
}

pub(super) fn available(context: &egui::Context) -> Vec<String> {
    context
        .data(|data| data.get_temp(egui::Id::new("code-fonts")))
        .unwrap_or_else(|| vec!["JetBrains Mono".into()])
}

pub(super) fn family(context: &egui::Context, name: &str) -> FontFamily {
    let known = context.data_mut(|data| {
        data.get_temp_mut_or_insert_with(egui::Id::new("code-fonts"), || {
            vec!["JetBrains Mono".to_owned()]
        })
        .iter()
        .any(|font| font == name)
    });
    if known {
        FontFamily::Name(name.into())
    } else {
        FontFamily::Monospace
    }
}

fn system_fonts() -> &'static [(&'static str, &'static str)] {
    if cfg!(target_os = "macos") {
        &[
            (
                "SF Mono",
                "/System/Applications/Utilities/Terminal.app/Contents/Resources/Fonts/SFMono-Regular.otf",
            ),
            ("Menlo", "/System/Library/Fonts/Menlo.ttc"),
        ]
    } else if cfg!(target_os = "windows") {
        &[
            ("Cascadia Mono", r"C:\Windows\Fonts\CascadiaMono.ttf"),
            ("Consolas", r"C:\Windows\Fonts\consola.ttf"),
        ]
    } else {
        &[
            (
                "DejaVu Sans Mono",
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            ),
            (
                "Liberation Mono",
                "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
            ),
        ]
    }
}
