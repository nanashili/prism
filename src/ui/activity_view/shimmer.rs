use crate::ui::theme::Palette;
use eframe::egui::{self, Rect, Shape, layers::ShapeIdx};
use std::sync::Arc;
use std::time::Duration;

const BAND_RADIUS: f32 = 60.0;

pub(in crate::ui) fn scope<R>(
    ui: &mut egui::Ui,
    active: bool,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if !active
        || !ui.is_visible()
        || !ui.input(|input| input.focused)
        || ui.style().animation_time <= 0.0
    {
        return content(ui);
    }

    let layer = ui.layer_id();
    let start = ui
        .ctx()
        .graphics(|graphics| graphics.get(layer).map_or(0, |list| list.next_idx().0));
    let bounds = ui.available_rect_before_wrap();
    let result = content(ui);

    let palette = Palette::for_context(ui.ctx());
    let phase = ui.input(|input| (input.time / 2.4).rem_euclid(1.0)) as f32;
    let sweep = bounds.left() - BAND_RADIUS + phase * (bounds.width() + BAND_RADIUS * 2.0);
    let visible = ui.ctx().graphics_mut(|graphics| {
        let Some(list) = graphics.get_mut(layer) else {
            return false;
        };
        let mut visible = false;
        for index in start..list.next_idx().0 {
            list.mutate_shape(ShapeIdx(index), |entry| {
                visible |= shade_shape(&mut entry.shape, entry.clip_rect, sweep, palette);
            });
        }
        visible
    });
    if visible {
        ui.ctx().request_repaint_after(Duration::from_millis(33));
    }
    result
}

fn shade_shape(shape: &mut Shape, clip: Rect, sweep: f32, palette: Palette) -> bool {
    match shape {
        Shape::Vec(shapes) => {
            let mut visible = false;
            for shape in shapes {
                visible |= shade_shape(shape, clip, sweep, palette);
            }
            visible
        }
        Shape::Text(text) if clip.intersects(text.visual_bounding_rect()) => {
            shade_text(text, clip, sweep, palette);
            true
        }
        _ => false,
    }
}

fn shade_text(text: &mut egui::epaint::TextShape, clip: Rect, sweep: f32, palette: Palette) {
    if text.angle != 0.0 {
        return;
    }
    let band = Rect::from_x_y_ranges(sweep - BAND_RADIUS..=sweep + BAND_RADIUS, clip.y_range());
    if !band.intersects(text.visual_bounding_rect()) {
        return;
    }

    let galley = Arc::make_mut(&mut text.galley);
    for row in &mut galley.rows {
        let offset = text.pos.to_vec2() + row.pos.to_vec2();
        let bounds = row.visuals.mesh_bounds.translate(offset);
        if !clip.intersects(bounds)
            || (!band.intersects(bounds) && text.override_text_color.is_none())
        {
            continue;
        }
        let visuals = &mut Arc::make_mut(&mut row.row).visuals;
        for vertex in &mut visuals.mesh.vertices[visuals.glyph_vertex_range.clone()] {
            let base = text.override_text_color.unwrap_or_else(|| {
                if vertex.color == egui::Color32::PLACEHOLDER {
                    text.fallback_color
                } else {
                    vertex.color
                }
            });
            let intensity = (1.0 - (vertex.pos.x + offset.x - sweep).abs() / BAND_RADIUS).max(0.0);
            let intensity = intensity * intensity * (3.0 - 2.0 * intensity);
            vertex.color = base.lerp_to_gamma(palette.text, intensity * 0.8);
        }
    }
    text.override_text_color = None;
}
