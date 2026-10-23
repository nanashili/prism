//! Prism's opt-in pane-fitting policy. Upstream's persistent widths stay default.

use egui::{Color32, Id, TextStyle, Ui};
use pulldown_cmark::Event;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

pub(super) fn minimum_width(ui: &Ui, cell: &[(Event<'_>, Range<usize>)]) -> f32 {
    cell.iter()
        .filter_map(|(event, _)| match event {
            Event::Code(text) => Some((text, TextStyle::Monospace)),
            Event::Text(text) => Some((text, TextStyle::Body)),
            _ => None,
        })
        .flat_map(|(text, style)| {
            let font = style.resolve(ui.style());
            text.unicode_words().map(move |word| {
                ui.painter()
                    .layout_no_wrap(word.to_owned(), font.clone(), Color32::WHITE)
                    .size()
                    .x
                    + 16.0
            })
        })
        .fold(40.0, f32::max)
}

pub(super) fn layout(
    ui: &Ui,
    table: Id,
    content_key: u64,
    fitted_widths: Vec<f32>,
    column_budget: f32,
) -> (Vec<f32>, bool) {
    let identity = table.with("_responsive_layout");
    let pixel_budget = (column_budget * ui.ctx().pixels_per_point()).round() as i64;
    let signature = (content_key, pixel_budget);
    let previous = ui.data(|data| data.get_temp::<(u64, i64)>(identity));
    let changed = previous != Some(signature);

    if changed {
        ui.data_mut(|data| data.insert_temp(identity, signature));
    }

    // The caller reuses resolved widths when unchanged, including manual
    // separator drags. A resize uses the new fit and resets TableBuilder's
    // previous widths before measuring and rendering rows.
    (fitted_widths, changed)
}
