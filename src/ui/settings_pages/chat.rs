use super::model::{Group, Preview, Row};
use crate::settings::Settings;
use crate::ui::presentation::{ActivityState, ChoiceAccess, ThoughtDisplay};
use eframe::egui;

const PREVIEW_REPLY: &str = r#"This sets `ready` to true, then prints its value.

```rust
let ready = true;
println!("Ready: {ready}");
```"#;

pub(super) fn groups(s: &mut Settings) -> Vec<Group<'_>> {
    let preview = s.clone();
    let c = &mut s.conversation;
    let mut groups = appearance_groups(&mut c.appearance, preview);
    groups.extend([
        Group::new("Replies", vec![
            Row::new("Show the agent’s thinking", "Folded under each reply. Open it to see how the agent got there.")
                .toggle(&mut c.show_thinking).blocked("Thinking is currently shown as live status. Folded thinking history is not available yet."),
            Row::new("How much of the chat the agent sees", "More context gives better answers on long chats but is slower.")
                .select(&mut c.history, &["Short", "Normal", "Long"]),
            Row::new("Open the workbench", "The full workbench next to the chat.")
                .select(&mut c.open_workbench, &["When the agent opens code", "Always", "Only when I ask"]),
        ]),
        Group::new("Composer", vec![
            Row::new("Send with", "The other one adds a new line.")
                .segmented(&mut c.send_key, &["Return", "Command Return"]),
        ]),
        Group::new("Notifications", vec![
            Row::new("Notify me when the agent finishes", "Show an in-app notification when a reply is ready or the agent stops.")
                .toggle(&mut c.notify_when_done),
            Row::new("Notify me when the agent needs my OK", "Show an in-app alert with a shortcut to the permission request in chat.")
                .toggle(&mut c.notify_when_asked),
            Row::new("Notify me when a file is ready", "Show an in-app confirmation when a project opens.")
                .toggle(&mut c.notify_when_ready),
            Row::new("Play a sound when the agent needs you", "For questions and permission requests.")
                .toggle(&mut c.sound_when_asked).blocked("Notification sounds are not available yet."),
        ]).foot("Notifications stay inside Prism. The bell keeps recent alerts until you quit."),
    ]);
    groups
}

fn appearance_groups(
    a: &mut crate::settings::chat_appearance::ChatAppearance,
    settings: Settings,
) -> Vec<Group<'_>> {
    let text = Group::new(
        "Text and blocks",
        vec![
            Row::new(
                "Chat font",
                "Choose the font used for messages and thinking.",
            )
            .select(&mut a.font, &["System", "Monospace"]),
            Row::new("Text size", "Messages, thinking and activity titles.").range(
                &mut a.text_size,
                12.0,
                22.0,
                1.0,
                " pt",
            ),
            Row::new(
                "Code size",
                "Code blocks and tool details, independently of the code view.",
            )
            .range(&mut a.code_size, 11.0, 20.0, 1.0, " pt"),
            Row::new(
                "Color chat blocks",
                "Syntax colors and subtle accents in quotes and activity details.",
            )
            .toggle(&mut a.block_colors),
        ],
    );
    let mut spacing = Group::new(
        "Spacing",
        vec![
            Row::new(
                "Line height",
                "Give wrapped message and code lines more room.",
            )
            .range(&mut a.line_spacing, 120.0, 190.0, 10.0, "%"),
            Row::new(
                "Paragraph spacing",
                "Space between paragraphs and blocks in a reply.",
            )
            .range(&mut a.paragraph_spacing, 4.0, 24.0, 1.0, " pt"),
            Row::new(
                "Message spacing",
                "Space between messages in the conversation.",
            )
            .range(&mut a.message_spacing, 12.0, 48.0, 2.0, " pt"),
        ],
    );
    spacing.preview = Some(Preview::Chat(settings));
    vec![text, spacing]
}

pub(super) fn preview(ui: &mut egui::Ui, settings: &Settings) {
    use crate::ui::message::{self, style::ChatStyle};

    let style = ChatStyle::new(ui.ctx(), settings);
    let palette = style.palette;
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Live preview")
                .size(12.5)
                .color(palette.muted),
        );
    });
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(palette.well)
        .corner_radius(14)
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            for (role, text) in [
                ("user", "Can you explain this code?"),
                ("assistant", PREVIEW_REPLY),
            ] {
                let sample = crate::model::Message {
                    id: None,
                    role: role.into(),
                    text: text.into(),
                    activity: (role == "assistant").then(preview_activity),
                };
                message::render_message(
                    ui,
                    &sample,
                    ThoughtDisplay::Visible,
                    ActivityState::Idle,
                    ChoiceAccess::Blocked,
                    &style,
                );
            }
        });
}

fn preview_activity() -> crate::model::Activity {
    use crate::model::{Activity, Block, Outcome, ToolStep};

    Activity {
        agent: "Prism".into(),
        started: 1_000,
        ended: Some(2_200),
        outcome: Outcome::Done,
        blocks: vec![
            Block::Thought {
                text: "Checking how the value is initialized and used.".into(),
            },
            Block::Tool(ToolStep {
                id: "preview-read".into(),
                title: "Read function".into(),
                status: Outcome::Done,
                started: 1_000,
                ended: Some(1_012),
                input: String::new(),
                output: "let ready = true;\nprintln!(\"Ready: {ready}\");".into(),
            }),
        ],
        limited: false,
        dropped_blocks: 0,
        dropped_thoughts: 0,
        dropped_text_end: 0,
        progress: Default::default(),
    }
}
