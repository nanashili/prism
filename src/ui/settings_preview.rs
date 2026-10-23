use super::theme::Palette;
use eframe::egui::{self, Color32, Rect, Sense, Stroke, pos2, vec2};

pub(super) fn window(ui: &mut egui::Ui, transparency: f32) {
    ui.add_space(12.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 168.0), Sense::hover());
    let p = Palette::for_context(ui.ctx());
    wallpaper(ui, rect);
    let painter = ui.painter().with_clip_rect(rect);
    let width = rect.width() * 0.84;
    let shell = Rect::from_min_size(
        pos2(rect.center().x - width * 0.5, rect.top() + 22.0),
        vec2(width, 160.0),
    );
    let alpha = ((1.0 - 0.7 * transparency / 100.0) * 255.0).round() as u8;
    painter.rect_filled(
        shell,
        10.0,
        Color32::from_rgba_unmultiplied(p.window.r(), p.window.g(), p.window.b(), alpha),
    );
    for (index, color) in [
        Color32::from_rgb(255, 95, 87),
        Color32::from_rgb(254, 188, 46),
        Color32::from_rgb(40, 200, 64),
    ]
    .iter()
    .enumerate()
    {
        painter.circle_filled(
            shell.min + vec2(14.0 + index as f32 * 13.0, 14.0),
            4.0,
            *color,
        );
    }
    let side_width = (width - 12.0) * 0.26;
    for (index, fraction) in [0.7, 0.55, 0.8, 0.6].into_iter().enumerate() {
        let y = 32.0 + index as f32 * 13.0;
        if index == 1 {
            painter.rect_filled(
                Rect::from_min_size(shell.min + vec2(6.0, y - 5.0), vec2(side_width, 15.0)),
                5.0,
                p.sel,
            );
        }
        painter.rect_filled(
            Rect::from_min_size(shell.min + vec2(10.0, y), vec2(side_width * fraction, 5.0)),
            3.0,
            p.muted.gamma_multiply(0.5),
        );
    }
    let card = Rect::from_min_max(
        shell.min + vec2(side_width + 12.0, 6.0),
        shell.max - vec2(6.0, 6.0),
    );
    painter.rect(
        card,
        8.0,
        p.card,
        Stroke::new(1.0, p.sel),
        egui::StrokeKind::Inside,
    );
    for (index, (fraction, color)) in [
        (0.34, Color32::from_rgb(198, 120, 221)),
        (0.62, p.line),
        (0.48, Color32::from_rgb(152, 195, 121)),
        (0.7, p.line),
    ]
    .iter()
    .enumerate()
    {
        painter.rect_filled(
            Rect::from_min_size(
                card.min + vec2(12.0, 12.0 + index as f32 * 12.0),
                vec2((card.width() - 24.0) * fraction, 5.0),
            ),
            3.0,
            *color,
        );
    }
}

fn wallpaper(ui: &egui::Ui, rect: Rect) {
    let texture = wallpaper_texture(ui.ctx());
    egui::Image::new((texture.id(), rect.size()))
        .corner_radius(12)
        .paint_at(ui, rect);

    let painter = ui.painter().with_clip_rect(rect);
    for (center, color) in [
        (
            pos2(rect.right() - 128.0, rect.top() + 36.0),
            Color32::from_rgb(63, 208, 201),
        ),
        (
            rect.min + vec2(88.0, 138.0),
            Color32::from_rgb(255, 95, 162),
        ),
    ] {
        for radius in (18..=54).rev().step_by(3) {
            painter.circle_filled(center, radius as f32, color.gamma_multiply(0.07));
        }
        painter.circle_filled(center, 18.0, color);
    }
}

fn wallpaper_texture(context: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::new("settings-wallpaper");
    if let Some(texture) = context.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }

    let purple = Color32::from_rgb(75, 59, 196);
    let orange = Color32::from_rgb(240, 135, 79);
    let midpoint = purple.lerp_to_gamma(orange, 0.5);
    let image = egui::ColorImage::new([2, 2], vec![purple, midpoint, midpoint, orange]);
    let texture = context.load_texture("settings-wallpaper", image, egui::TextureOptions::LINEAR);
    context.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}
