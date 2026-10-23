use crate::ui::{app::Prism, theme::Palette};
use eframe::egui::{self, Sense, Stroke};

impl Prism {
    pub(in crate::ui) fn context_usage(&self, ui: &mut egui::Ui) {
        let Some(usage) = &self.chat.usage else {
            return;
        };
        if usage.size == 0 {
            return;
        }
        let ratio = (usage.used as f64 / usage.size as f64).clamp(0.0, 1.0) as f32;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(24.0, 30.0), Sense::hover());
        let p = Palette::for_context(ui.ctx());
        ui.painter()
            .circle_stroke(rect.center(), 6.5, Stroke::new(2.0, p.line));
        let points: Vec<_> = (0..=32)
            .map(|step| {
                let angle = -std::f32::consts::FRAC_PI_2
                    + std::f32::consts::TAU * ratio * step as f32 / 32.0;
                rect.center() + egui::vec2(angle.cos(), angle.sin()) * 6.5
            })
            .collect();
        ui.painter().add(egui::Shape::line(
            points,
            Stroke::new(
                2.0,
                if ratio > 0.9 {
                    ui.visuals().warn_fg_color
                } else {
                    p.muted
                },
            ),
        ));
        response.on_hover_ui(|ui| {
            ui.label(format!("Context · {:.0}% used", ratio * 100.0));
            ui.weak(format!("{} of {} tokens", usage.used, usage.size));
            if let Some(cost) = usage.cost {
                ui.weak(format!("Cost · {cost:.4} {}", usage.currency));
            }
        });
    }
}
