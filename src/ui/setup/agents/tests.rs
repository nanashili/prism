use super::*;
use egui::{Event, PointerButton, Pos2, Rect, Shape};

struct Picker {
    context: egui::Context,
    catalog: Catalog,
    settings: Settings,
    action: Option<Command>,
}

impl Picker {
    fn draw(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.context.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(760.0, 800.0))),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    if let Some(action) = show(ui, &mut self.catalog, &mut self.settings) {
                        self.action = Some(action);
                    }
                });
            },
        )
    }

    fn click(&mut self, position: Pos2) {
        for pressed in [true, false] {
            self.draw(vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
    }
}

fn text_position(output: &egui::FullOutput, text: &str) -> Option<Pos2> {
    output.shapes.iter().find_map(|shape| {
        if let Shape::Text(shape) = &shape.shape
            && shape.galley.text() == text
        {
            return Some(shape.pos + shape.galley.size() * 0.5);
        }
        None
    })
}

#[test]
fn setup_offers_detected_agents_and_emits_selection_and_rescan_actions() {
    let mut catalog = Catalog::default();
    catalog.entries = crate::agent::catalog::fallback();
    catalog.installed = [
        "claude-acp".into(),
        "codex-acp".into(),
        "github-copilot-cli".into(),
    ]
    .into();
    catalog.scanned = true;
    let mut picker = Picker {
        context: egui::Context::default(),
        catalog,
        settings: Settings::default(),
        action: None,
    };
    picker.settings.agent_id = "codex-acp".into();
    picker.settings.agent_name = "Codex".into();
    let output = picker.draw(Vec::new());

    assert!(text_position(&output, "Installed · Selected").is_some());
    assert!(
        text_position(&output, "Gemini").is_none(),
        "Undetected agents start collapsed"
    );
    assert!(text_position(&output, "Enter the command for an agent that supports ACP.").is_none());
    let copilot = text_position(&output, "GitHub Copilot").expect("Show the installed Copilot CLI");
    picker.click(copilot);
    assert!(matches!(&picker.action, Some(Command::Agent(id, _)) if id == "github-copilot-cli"));
    assert_eq!(
        picker.settings.agent_id, "codex-acp",
        "Selection is applied through the shared command handler"
    );

    let scan = text_position(&output, "Scan again").expect("Offer to refresh discovery");
    picker.click(scan);
    assert!(matches!(picker.action, Some(Command::Rescan)));

    picker.catalog.installed.clear();
    let empty = picker.draw(Vec::new());
    assert!(text_position(&empty, "No agents found on this computer").is_some());
    assert!(text_position(&empty, "Selected · Not installed").is_some());
    assert!(text_position(&empty, "Installed · Selected").is_none());
}
