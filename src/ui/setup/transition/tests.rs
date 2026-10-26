use super::*;
use egui::{Event, PointerButton, Pos2, Shape};

#[derive(Default)]
struct Panel {
    context: egui::Context,
    selections: usize,
}

impl Panel {
    fn draw(
        &mut self,
        time: f64,
        step: usize,
        duration: f32,
        events: Vec<Event>,
    ) -> egui::FullOutput {
        self.context.run(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(760.0, 500.0))),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    show(ui, ui.max_rect(), step, duration, |ui, displayed| {
                        ui.label(format!("Step {displayed}"));
                        if ui.button("Choose").clicked() {
                            self.selections += 1;
                        }
                    });
                });
            },
        )
    }

    fn click(&mut self, time: f64, step: usize, position: Pos2, duration: f32) {
        for (index, pressed) in [true, false].into_iter().enumerate() {
            self.draw(
                time + index as f64 * 0.001,
                step,
                duration,
                vec![
                    Event::PointerMoved(position),
                    Event::PointerButton {
                        pos: position,
                        button: PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
}

fn text(output: &egui::FullOutput, label: &str) -> Option<(Rect, u8)> {
    output.shapes.iter().find_map(|shape| {
        if let Shape::Text(shape) = &shape.shape
            && shape.galley.text() == label
        {
            Some((
                Rect::from_min_size(shape.pos, shape.galley.size()),
                shape.fallback_color.a(),
            ))
        } else {
            None
        }
    })
}

#[test]
fn steps_fade_in_direction_and_ignore_content_clicks_until_settled() {
    let mut panel = Panel::default();
    let initial = panel.draw(0.0, 0, 0.2, Vec::new());
    let (start, alpha) = text(&initial, "Step 0").expect("First step is immediately visible");
    let button = text(&initial, "Choose")
        .expect("the initial choice is visible")
        .0
        .center();

    panel.draw(1.0, 2, 0.2, Vec::new());
    let leaving = panel.draw(1.02, 2, 0.2, Vec::new());
    let (old, faded) = text(&leaving, "Step 0").expect("Keep the outgoing step during its exit");
    assert!(old.left() < start.left());
    assert!(faded > 0 && faded < alpha);
    assert!(text(&leaving, "Step 2").is_none());
    panel.click(1.03, 2, button, 0.2);
    assert_eq!(panel.selections, 0, "Moving content cannot receive clicks");

    let entering = panel.draw(1.10, 2, 0.2, Vec::new());
    let (new, faded) = text(&entering, "Step 2").expect("The next step enters from the right");
    assert!(new.left() > start.left());
    assert!(faded > 0 && faded < alpha);
    assert!(text(&entering, "Step 0").is_none());

    let settled = panel.draw(1.25, 2, 0.2, Vec::new());
    assert_eq!(text(&settled, "Step 2"), Some((start, alpha)));
    panel.click(1.30, 2, button, 0.2);
    assert_eq!(panel.selections, 1);

    panel.draw(2.0, 1, 0.2, Vec::new());
    let backward = panel.draw(2.10, 1, 0.2, Vec::new());
    assert!(
        text(&backward, "Step 1")
            .expect("the previous step is entering")
            .0
            .left()
            < start.left()
    );
    panel.draw(2.11, 4, 0.2, Vec::new());
    let redirected = panel.draw(2.40, 4, 0.2, Vec::new());
    assert_eq!(text(&redirected, "Step 4"), Some((start, alpha)));
    assert!(text(&redirected, "Step 1").is_none());
}

#[test]
fn disabling_motion_finishes_an_active_transition_immediately() {
    let mut panel = Panel::default();
    let initial = panel.draw(0.0, 0, 0.2, Vec::new());
    let original = text(&initial, "Step 0").expect("the initial step is visible");

    panel.draw(1.0, 3, 0.2, Vec::new());
    let reduced = panel.draw(1.01, 3, 0.0, Vec::new());
    assert_eq!(text(&reduced, "Step 3"), Some(original));
    assert!(text(&reduced, "Step 0").is_none());

    let button = text(&reduced, "Choose")
        .expect("the settled choice is visible")
        .0
        .center();
    panel.click(1.02, 3, button, 0.0);
    assert_eq!(panel.selections, 1);
    let next = panel.draw(1.10, 1, 0.0, Vec::new());
    assert_eq!(text(&next, "Step 1"), Some(original));
}
