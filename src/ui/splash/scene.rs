use super::geometry::Layout;
use eframe::egui::{self, Color32, Mesh, Painter, Pos2, Shape, Stroke};

const SPECTRUM: [Color32; 7] = [
    Color32::from_rgb(255, 99, 111),
    Color32::from_rgb(255, 164, 91),
    Color32::from_rgb(255, 222, 126),
    Color32::from_rgb(123, 229, 169),
    Color32::from_rgb(101, 212, 246),
    Color32::from_rgb(116, 152, 255),
    Color32::from_rgb(188, 139, 255),
];

struct Scene<'a> {
    painter: &'a Painter,
    center: Pos2,
    scale: f32,
}

pub(super) fn paint(painter: &Painter, glass: Option<egui::TextureId>, layout: &Layout, time: f32) {
    let scene = Scene {
        painter,
        center: layout.center,
        scale: layout.scale,
    };
    if let Some(texture) = glass {
        scene.glass(texture);
    }
    scene.incident_light(super::progress(time, 0.35, 1.2));
    scene.refraction(
        super::progress(time, 1.2, 1.65),
        super::progress(time, 1.65, 2.65),
    );
    scene.highlights(
        super::progress(time, 1.2, 1.4),
        super::progress(time, 1.65, 1.85),
    );
}

impl Scene<'_> {
    fn point(&self, x: f32, y: f32) -> Pos2 {
        self.center + egui::vec2(x, y) * self.scale
    }

    fn glass(&self, texture: egui::TextureId) {
        let bounds = egui::Rect::from_min_max(self.point(-165.0, -180.0), self.point(165.0, 150.0));
        let uv = egui::Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0));
        self.painter.image(texture, bounds, uv, Color32::WHITE);
    }

    fn incident_light(&self, progress: f32) {
        if progress == 0.0 {
            return;
        }
        let start = self.point(-440.0, 42.0);
        let entry = self.point(-86.0, -18.0);
        let end = start.lerp(entry, progress);
        let white = Color32::from_rgb(232, 243, 255);

        self.ribbon(start, end, 0.7, 0.7, white, progress);
        self.beam(start, end, white, progress);
    }

    fn refraction(&self, through: f32, spread: f32) {
        let entry = self.point(-86.0, -18.0);
        let exit = self.point(96.0, 2.0);
        if through > 0.0 {
            self.beam(
                entry,
                entry.lerp(exit, through),
                Color32::from_rgb(214, 233, 255),
                through * 0.8,
            );
        }
        if spread == 0.0 {
            return;
        }

        for (index, color) in SPECTRUM.iter().enumerate() {
            let offset = index as f32;
            let source = exit + egui::vec2(0.4 * offset, 0.7 * offset) * self.scale;
            let end = source.lerp(self.point(440.0, 62.0 + offset * 16.0), spread);
            self.ribbon(source, end, 0.8, 9.0 * spread, *color, 0.34 * spread);
            self.beam(source, end, *color, spread);
        }
    }

    fn highlights(&self, entry_light: f32, exit_light: f32) {
        if entry_light == 0.0 {
            return;
        }
        let entry = self.point(-86.0, -18.0);
        let exit = self.point(96.0, 2.0);
        for (point, strength) in [(entry, entry_light), (exit, exit_light * 0.8)] {
            for radius in [16.0, 10.0, 5.0] {
                self.painter.circle_filled(
                    point,
                    radius * self.scale,
                    Color32::from_rgb(171, 210, 255).gamma_multiply(strength * 0.035),
                );
            }
            self.painter.circle_filled(
                point,
                1.7 * self.scale,
                Color32::WHITE.gamma_multiply(strength),
            );
        }
    }

    fn beam(&self, start: Pos2, end: Pos2, color: Color32, opacity: f32) {
        for (width, strength) in [(10.0, 0.035), (5.0, 0.07), (2.4, 0.2), (1.0, 0.9)] {
            self.painter.line_segment(
                [start, end],
                Stroke::new(width * self.scale, color.gamma_multiply(opacity * strength)),
            );
        }
    }

    fn ribbon(
        &self,
        start: Pos2,
        end: Pos2,
        start_width: f32,
        end_width: f32,
        color: Color32,
        opacity: f32,
    ) {
        let normal = (end - start).normalized().rot90() * self.scale;
        self.face(
            &[
                start - normal * start_width,
                start + normal * start_width,
                end + normal * end_width,
                end - normal * end_width,
            ],
            &[
                color.gamma_multiply(opacity),
                color.gamma_multiply(opacity),
                Color32::TRANSPARENT,
                Color32::TRANSPARENT,
            ],
        );
    }

    fn face(&self, points: &[Pos2], colors: &[Color32]) {
        let mut mesh = Mesh::default();
        for (&point, &color) in points.iter().zip(colors) {
            mesh.colored_vertex(point, color);
        }
        for index in 1..points.len() as u32 - 1 {
            mesh.add_triangle(0, index, index + 1);
        }
        self.painter.add(Shape::mesh(mesh));
    }
}
