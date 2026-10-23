use eframe::egui::{self, Pos2, Rect};

#[cfg(target_os = "macos")]
pub(super) const SILHOUETTE: [(f32, f32); 5] = [
    (-20.0, -160.0),
    (14.0, -174.0),
    (144.0, 100.0),
    (110.0, 125.0),
    (-155.0, 125.0),
];

#[cfg(target_os = "macos")]
pub(super) const FACET: [(f32, f32); 4] = [
    (-20.0, -160.0),
    (14.0, -174.0),
    (144.0, 100.0),
    (110.0, 125.0),
];

pub(super) struct Layout {
    #[cfg(target_os = "macos")]
    pub viewport: Rect,
    pub center: Pos2,
    pub scale: f32,
}

impl Layout {
    pub fn new(viewport: Rect) -> Self {
        let scale = (viewport.width() / 1000.0)
            .min(viewport.height() / 660.0)
            .clamp(0.4, 1.2);
        Self {
            #[cfg(target_os = "macos")]
            viewport,
            center: viewport.center() - egui::vec2(0.0, 48.0 * scale),
            scale,
        }
    }

    #[cfg(target_os = "macos")]
    pub fn prism_bounds(&self) -> Rect {
        Rect::from_min_max(self.point(-165.0, -180.0), self.point(165.0, 150.0))
    }

    #[cfg(target_os = "macos")]
    pub fn point(&self, x: f32, y: f32) -> Pos2 {
        self.center + egui::vec2(x, y) * self.scale
    }
}
