use super::app::Prism;
use eframe::egui::{self, Color32, FontId, Rect, RichText};
use std::time::{Duration, Instant};

mod geometry;
mod material;
#[cfg(target_os = "macos")]
mod native;
mod scene;

const DURATION: f32 = 4.2;
const BACKGROUND: Color32 = Color32::from_rgb(7, 9, 13);

pub(super) struct Splash {
    started: Option<Instant>,
    glass: material::Material,
}

impl Prism {
    pub(super) fn show_splash(&mut self, context: &egui::Context) -> bool {
        let Some(splash) = &mut self.splash else {
            return false;
        };
        if splash.show(context) {
            self.finish_splash();
        }
        true
    }

    fn finish_splash(&mut self) {
        self.splash = None;
        self.settings.desktop.intro_seen = true;

        if let Err(error) = self.settings.save() {
            self.settings.desktop.intro_seen = false;
            self.report_error(format!("Couldn't save the startup preference: {error}"));
        }
    }
}

impl Splash {
    pub(super) fn new(context: &eframe::CreationContext<'_>) -> crate::error::Result<Self> {
        Ok(Self {
            started: None,
            glass: material::Material::new(context)?,
        })
    }

    fn show(&mut self, context: &egui::Context) -> bool {
        let first_frame = self.started.is_none();
        let elapsed = self.started.get_or_insert_with(Instant::now).elapsed();
        let seconds = elapsed.as_secs_f32();
        let animations = context.style().animation_time > 0.0;
        let finished = seconds >= DURATION || !animations;
        let time = if animations { seconds } else { 2.8 };

        let skipped = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BACKGROUND))
            .show(context, |ui| paint(ui, &mut self.glass, time, first_frame))
            .inner;

        if finished || skipped {
            context.request_repaint();
        } else {
            context.request_repaint_after(Duration::from_millis(16));
        }
        finished || skipped
    }
}

fn paint(ui: &mut egui::Ui, glass: &mut material::Material, time: f32, first_frame: bool) -> bool {
    let bounds = ui.max_rect();
    let layout = geometry::Layout::new(bounds);
    let opacity = progress(time, 0.0, 0.55) * (1.0 - progress(time, 3.75, DURATION));
    let mut painter = ui.painter().clone();
    painter.set_opacity(opacity);

    glass.update(&layout, opacity, ui.ctx().pixels_per_point());
    scene::paint(&painter, glass.texture(), &layout, time);
    wordmark(&painter, layout.center, layout.scale, time);
    skip_button(ui, bounds, first_frame)
}

fn wordmark(painter: &egui::Painter, center: egui::Pos2, scale: f32, time: f32) {
    let mut painter = painter.clone();
    painter.multiply_opacity(progress(time, 1.65, 2.45));
    painter.text(
        center + egui::vec2(0.0, 190.0 * scale),
        egui::Align2::CENTER_CENTER,
        "Prism",
        FontId::proportional(34.0 * scale),
        Color32::from_rgb(237, 242, 250),
    );
    painter.text(
        center + egui::vec2(0.0, 226.0 * scale),
        egui::Align2::CENTER_CENTER,
        "See what's inside.",
        FontId::proportional(13.0 * scale),
        Color32::from_rgb(143, 154, 172),
    );
}

fn skip_button(ui: &mut egui::Ui, bounds: Rect, first_frame: bool) -> bool {
    let rect = Rect::from_center_size(
        egui::pos2(bounds.center().x, bounds.bottom() - 42.0),
        egui::vec2(104.0, 32.0),
    );
    let response = ui.put(
        rect,
        egui::Button::new(
            RichText::new("Skip intro")
                .size(12.0)
                .color(Color32::from_rgb(153, 163, 180)),
        )
        .fill(BACKGROUND)
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(40, 46, 58)))
        .corner_radius(8),
    );
    if first_frame {
        response.request_focus();
    }
    response.clicked()
}

fn progress(time: f32, start: f32, end: f32) -> f32 {
    let fraction = ((time - start) / (end - start)).clamp(0.0, 1.0);
    fraction * fraction * (3.0 - 2.0 * fraction)
}
