use crate::agent::options::OptionMenu;
use crate::ui::theme::Palette;
use eframe::egui::{self, Color32, Rect, Stroke};

pub(super) fn show(ui: &mut egui::Ui, menu: &OptionMenu, model: Option<&str>) -> (usize, bool) {
    let current = menu
        .choices
        .iter()
        .position(|choice| choice.value == menu.current)
        .unwrap_or(0);
    let draft_id = ui.id().with(("reasoning-drag", &menu.id, model));
    let mut index = ui
        .ctx()
        .data(|data| data.get_temp::<usize>(draft_id))
        .unwrap_or(current)
        .min(menu.choices.len() - 1);
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().slider_width = ui.available_width();
            ui.spacing_mut().interact_size.y = 32.0;
            let visuals = ui.visuals_mut();
            visuals.selection.bg_fill = Color32::TRANSPARENT;
            for widget in [
                &mut visuals.widgets.inactive,
                &mut visuals.widgets.hovered,
                &mut visuals.widgets.active,
            ] {
                widget.bg_fill = Color32::TRANSPARENT;
                widget.fg_stroke = Stroke::NONE;
            }
            ui.add_enabled(
                menu.choices.len() > 1,
                egui::Slider::new(&mut index, 0..=menu.choices.len() - 1)
                    .show_value(false)
                    .smart_aim(false)
                    .handle_shape(egui::style::HandleShape::Circle),
            )
        })
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::slider(
            ui.is_enabled(),
            index as f64,
            format!("Reasoning effort: {}", menu.choices[index].name),
        )
    });
    paint_slider(
        ui,
        response.rect,
        index,
        menu.choices.len(),
        response.has_focus(),
    );
    let commit =
        !response.dragged() && (response.changed() || response.drag_stopped()) && index != current;
    ui.ctx().data_mut(|data| {
        if response.dragged() {
            data.insert_temp(draft_id, index);
        } else {
            data.remove::<usize>(draft_id);
        }
    });
    response.on_hover_text(&menu.choices[index].description);
    (index, commit)
}

fn paint_slider(ui: &egui::Ui, bounds: Rect, index: usize, count: usize, focused: bool) {
    let p = Palette::for_context(ui.ctx());
    let track = Rect::from_center_size(bounds.center(), egui::vec2(bounds.width(), 24.0));
    let radius = bounds.height() / 2.5;
    let positions = bounds.x_range().shrink(radius);
    let fraction = |index: usize| {
        if count > 1 {
            index as f32 / (count - 1) as f32
        } else {
            0.0
        }
    };
    let x = egui::lerp(positions, fraction(index));
    ui.painter().rect_filled(track, 12.0, p.line3);
    paint_spectrum(ui, track, x + radius);
    for tick in 0..count {
        if tick != index {
            ui.painter().circle_filled(
                egui::pos2(egui::lerp(positions, fraction(tick)), track.center().y),
                2.0,
                if tick < index {
                    Color32::from_black_alpha(90)
                } else {
                    p.text.gamma_multiply(0.45)
                },
            );
        }
    }
    ui.painter()
        .circle_filled(egui::pos2(x, track.center().y), radius, Color32::WHITE);
    if focused {
        ui.painter().rect_stroke(
            track.expand(3.0),
            15.0,
            Stroke::new(1.0, p.accent),
            egui::StrokeKind::Outside,
        );
    }
}

fn paint_spectrum(ui: &egui::Ui, track: Rect, end: f32) {
    let texture = spectrum_texture(ui.ctx());
    let fill = Rect::from_min_max(track.min, egui::pos2(end, track.bottom()));
    let fraction = (fill.width() / track.width()).clamp(0.0, 1.0);
    let uv = Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(fraction, 1.0));
    ui.painter().add(
        egui::epaint::RectShape::filled(fill, 12.0, Color32::WHITE).with_texture(texture.id(), uv),
    );
}

fn spectrum_texture(context: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::new("reasoning-prism-spectrum");
    if let Some(texture) = context.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }

    let colors = vec![
        Color32::from_rgb(109, 132, 247),
        Color32::from_rgb(75, 167, 238),
        Color32::from_rgb(69, 204, 215),
        Color32::from_rgb(110, 211, 153),
        Color32::from_rgb(239, 212, 104),
        Color32::from_rgb(243, 161, 99),
        Color32::from_rgb(231, 112, 137),
    ];
    let image = egui::ColorImage::new([colors.len(), 1], colors);
    let texture = context.load_texture(
        "reasoning-prism-spectrum",
        image,
        egui::TextureOptions::LINEAR,
    );
    context.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}
