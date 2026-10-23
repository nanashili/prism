use super::*;
use egui::{Event, PointerButton, Pos2, Shape};

struct Keyboard {
    context: egui::Context,
    settings: Settings,
    editor: Editor,
}

impl Keyboard {
    fn draw(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.context.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(720.0, 800.0))),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    show(ui, &mut self.settings, &mut self.editor);
                });
            },
        )
    }

    fn click(&mut self, position: Pos2) -> egui::FullOutput {
        for pressed in [true, false] {
            self.draw(vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        self.draw(Vec::new())
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
fn keyboard_tracks_modifier_clicks_presets_and_custom_bindings() {
    let mut keyboard = Keyboard {
        context: egui::Context::default(),
        settings: Settings::default(),
        editor: Editor::default(),
    };
    let output = keyboard.draw(Vec::new());
    assert!(text_position(&output, "Open").is_some());
    assert!(text_position(&output, "Rename").is_none());

    let command = text_position(
        &output,
        if cfg!(target_os = "macos") {
            "command"
        } else {
            "ctrl"
        },
    )
    .expect("Command key is visible");
    let plain = keyboard.click(command);
    let rename = text_position(&plain, "Rename").expect("Plain N displays Rename");
    let n = text_position(&plain, "N").expect("N key is visible");
    assert!((rename.x - n.x).abs() < 1.0);
    assert!(text_position(&plain, "Open").is_none());

    keyboard.settings.views.keymap = "Ghidra".into();
    let ghidra = keyboard.draw(Vec::new());
    let rename = text_position(&ghidra, "Rename").expect("Ghidra moves Rename to L");
    let l = text_position(&ghidra, "L").expect("L key is visible");
    assert!((rename.x - l.x).abs() < 1.0);

    keyboard
        .settings
        .views
        .shortcuts
        .insert("rename".into(), "alt-k".into());
    let output = keyboard.draw(Vec::new());
    assert!(text_position(&output, "Rename").is_none());
    let option = text_position(
        &output,
        if cfg!(target_os = "macos") {
            "option"
        } else {
            "alt"
        },
    )
    .expect("Option key is visible");
    let alternate = keyboard.click(option);
    let rename = text_position(&alternate, "Rename").expect("Custom Option-K displays Rename");
    let k = text_position(&alternate, "K").expect("K key is visible");
    assert!((rename.x - k.x).abs() < 1.0);

    let selected = keyboard.click(k);
    assert!(text_position(&selected, "K shortcuts").is_some());
    assert_eq!(keyboard.settings.views.shortcuts["rename"], "alt-k");
}
