use super::style::ChatStyle;
use eframe::egui;
use egui_commonmark_extended::{CommonMarkCache, CommonMarkViewer};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use std::borrow::Cow;

pub(super) fn show(ui: &mut egui::Ui, text: &str, style: &ChatStyle) {
    let rendered = if style.appearance.block_colors {
        Cow::Borrowed(text)
    } else {
        without_syntax_hints(text)
    };

    show_cached(ui, &rendered, style, &mut CommonMarkCache::default());
}

pub(super) fn reset_layout(cache: &mut CommonMarkCache, text: &str) {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();
    let events = cache.get_cached_events(hash).map(<[_]>::to_vec);
    *cache = CommonMarkCache::default();
    if let Some(events) = events {
        cache.set_cached_events(hash, events);
    }
}

pub(super) fn show_cached(
    ui: &mut egui::Ui,
    text: &str,
    style: &ChatStyle,
    cache: &mut CommonMarkCache,
) {
    let width = ui.available_width().max(1.0) as usize;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = style.appearance.paragraph_spacing * 0.5;
        let visuals = ui.visuals_mut();
        visuals.code_bg_color = style.palette.fill;
        visuals.extreme_bg_color = style.palette.fill;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

        CommonMarkViewer::new()
            .explicit_image_uri_scheme(true)
            .default_width(Some(width))
            .table_max_width(Some(width))
            .responsive_tables(true)
            .max_image_width(Some(width))
            .line_height(style.appearance.line_spacing / 100.0)
            .code_line_height(style.appearance.line_spacing / 100.0)
            .paragraph_spacing_px(style.appearance.paragraph_spacing)
            .heading_spacing_above_px(style.appearance.paragraph_spacing)
            .heading_spacing_below_px(style.appearance.paragraph_spacing * 0.5)
            .show(ui, cache, text);
    });
}

pub(super) fn without_syntax_hints(text: &str) -> Cow<'_, str> {
    let hints: Vec<_> = Parser::new(text)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) = event else {
                return None;
            };
            if language.is_empty() {
                return None;
            }

            let opening = text.get(range.clone())?.split('\n').next()?;
            let fence_start = opening.find(['`', '~'])?;
            let fence = opening.as_bytes()[fence_start];
            let fence_len = opening.as_bytes()[fence_start..]
                .iter()
                .take_while(|byte| **byte == fence)
                .count();
            Some(range.start + fence_start + fence_len..range.start + opening.len())
        })
        .collect();

    let mut rendered = Cow::Borrowed(text);
    for hint in hints.into_iter().rev() {
        rendered.to_mut().replace_range(hint, "");
    }
    rendered
}
