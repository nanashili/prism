use super::style::ChatStyle;
use eframe::egui;

pub(in crate::ui) fn code_text(ui: &mut egui::Ui, text: &str, style: &ChatStyle) {
    let mut job = egui::text::LayoutJob::default();
    let scheme = if style.appearance.block_colors {
        "Prism"
    } else {
        "Quiet"
    };
    for line in text.split_inclusive('\n') {
        let highlighted =
            crate::ui::syntax::highlight(line, style.code_font.clone(), style.dark, scheme);
        for section in highlighted.sections {
            let mut format = section.format;
            if format.color == egui::Color32::from_rgb(92, 99, 112)
                || format.color == egui::Color32::from_rgb(160, 161, 167)
                || format.color == egui::Color32::from_gray(107)
            {
                format.color = style.palette.muted;
            }
            format.line_height = Some(style.appearance.line_height(style.code_font.size));
            job.append(&line[section.byte_range], 0.0, format);
        }
    }
    ui.add(egui::Label::new(job).selectable(true).wrap());
}
