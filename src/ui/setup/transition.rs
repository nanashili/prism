use eframe::egui::{self, Rect, emath::TSTransform};

const DISTANCE: f32 = 12.0;
const EXIT_FRACTION: f32 = 0.35;

#[derive(Clone, Copy)]
struct Pose {
    opacity: f32,
    offset: f32,
}

impl Pose {
    const VISIBLE: Self = Self {
        opacity: 1.0,
        offset: 0.0,
    };

    fn toward(self, target: Self, progress: f32) -> Self {
        Self {
            opacity: egui::lerp(self.opacity..=target.opacity, progress),
            offset: egui::lerp(self.offset..=target.offset, progress),
        }
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Idle,
    Leaving { started: f64, from: Pose },
    Entering { started: f64, from: Pose },
}

#[derive(Clone, Copy)]
struct Transition {
    step: usize,
    target: usize,
    phase: Phase,
}

impl Transition {
    fn new(step: usize) -> Self {
        Self {
            step,
            target: step,
            phase: Phase::Idle,
        }
    }

    fn update(&mut self, requested: usize, now: f64, duration: f32) {
        if duration <= 0.0 {
            *self = Self::new(requested);
            return;
        }

        self.finish_phases(now, duration);
        if requested == self.target {
            return;
        }

        let from = self.pose(now, duration);
        self.target = requested;
        self.phase = if requested == self.step {
            Phase::Entering { started: now, from }
        } else {
            Phase::Leaving { started: now, from }
        };
    }

    fn finish_phases(&mut self, now: f64, duration: f32) {
        let exit_duration = f64::from(duration * EXIT_FRACTION);
        if let Phase::Leaving { started, .. } = self.phase
            && now >= started + exit_duration
        {
            let offset = self.direction() * DISTANCE;
            self.step = self.target;
            self.phase = Phase::Entering {
                started: started + exit_duration,
                from: Pose {
                    opacity: 0.0,
                    offset,
                },
            };
        }

        let enter_duration = f64::from(duration * (1.0 - EXIT_FRACTION));
        if let Phase::Entering { started, .. } = self.phase
            && now >= started + enter_duration
        {
            self.phase = Phase::Idle;
        }
    }

    fn pose(&self, now: f64, duration: f32) -> Pose {
        let (started, from, target, fraction) = match self.phase {
            Phase::Idle => return Pose::VISIBLE,
            Phase::Leaving { started, from } => (
                started,
                from,
                Pose {
                    opacity: 0.0,
                    offset: -self.direction() * DISTANCE,
                },
                EXIT_FRACTION,
            ),
            Phase::Entering { started, from } => {
                (started, from, Pose::VISIBLE, 1.0 - EXIT_FRACTION)
            }
        };
        let elapsed = (now - started) as f32;
        let progress = (elapsed / (duration * fraction)).clamp(0.0, 1.0);
        from.toward(target, egui::emath::easing::cubic_out(progress))
    }

    fn direction(&self) -> f32 {
        if self.target > self.step { 1.0 } else { -1.0 }
    }

    fn active(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }
}

pub(super) fn duration(ui: &egui::Ui) -> f32 {
    if ui.style().animation_time <= 0.0 {
        return 0.0;
    }

    #[cfg(target_os = "macos")]
    if objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion() {
        return 0.0;
    }

    0.2
}

pub(super) fn show(
    ui: &mut egui::Ui,
    bounds: Rect,
    requested: usize,
    duration: f32,
    content: impl FnOnce(&mut egui::Ui, usize),
) {
    let id = ui.id().with("setup-transition");
    let now = ui.input(|input| input.time);
    let mut transition = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(id, || Transition::new(requested)));
    transition.update(requested, now, duration);
    ui.ctx().data_mut(|data| data.insert_temp(id, transition));

    if transition.active() {
        ui.ctx().request_repaint();
    }

    let pose = transition.pose(now, duration);
    ui.scope_builder(egui::UiBuilder::new().max_rect(bounds), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(bounds));
        let opacity = ui.opacity();
        if transition.active() {
            ui.disable();
        }
        ui.set_opacity(opacity * pose.opacity);
        ui.with_visual_transform(
            TSTransform::from_translation(egui::vec2(pose.offset, 0.0)),
            |ui| content(ui, transition.step),
        );
    });
}

#[cfg(test)]
mod tests;
