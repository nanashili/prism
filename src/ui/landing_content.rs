use crate::ui::presentation::Interaction;
use eframe::egui;
use std::time::Duration;

const INTERVAL: f64 = 12.0;

pub(super) struct Content {
    pub title: &'static str,
    pub prompts: [&'static str; 4],
}

#[derive(Default)]
pub(super) struct Rotation {
    index: usize,
    deadline: Option<f64>,
}

impl Rotation {
    pub fn content(&self) -> &'static Content {
        &CONTENTS[self.index]
    }

    pub fn pause(&mut self) {
        self.deadline = None;
    }

    pub fn update(&mut self, context: &egui::Context, interacting: Interaction) {
        let interacting = interacting == Interaction::Active;

        let (now, focused, holding) =
            context.input(|input| (input.time, input.focused, input.pointer.any_down()));
        if interacting
            || !focused
            || holding
            || context.wants_keyboard_input()
            || egui::Popup::is_any_open(context)
        {
            self.pause();
            return;
        }

        let deadline = *self.deadline.get_or_insert(now + INTERVAL);
        if now >= deadline {
            self.index = (self.index + 1) % CONTENTS.len();
            self.deadline = Some(now + INTERVAL);
            context.request_repaint();
            return;
        }

        context.request_repaint_after(Duration::from_secs_f64(deadline - now));
    }
}

const CONTENTS: [Content; 8] = [
    Content {
        title: "What should we take apart?",
        prompts: [
            "What does /bin/ls do when it starts?",
            "How does Notes save attachments?",
            "Is this .pt file safe to load?",
            "Why is this blur shader slow?",
        ],
    },
    Content {
        title: "Where does the story begin?",
        prompts: [
            "Find this binary's entry point",
            "What happens when this app launches?",
            "What inputs does this model expect?",
            "Walk me through this shader",
        ],
    },
    Content {
        title: "Follow the data",
        prompts: [
            "Where does this binary read its config?",
            "Where does this app store its data?",
            "What tensors are inside this model?",
            "How does this shader sample textures?",
        ],
    },
    Content {
        title: "Find what slows things down",
        prompts: [
            "Which functions do the most work?",
            "Why does this app take so long to open?",
            "Which layers use the most compute?",
            "Where does this shader waste work?",
        ],
    },
    Content {
        title: "Look a little closer",
        prompts: [
            "What libraries does this binary use?",
            "What network requests does this app make?",
            "How are this model's weights stored?",
            "What do these shader constants control?",
        ],
    },
    Content {
        title: "Make every byte count",
        prompts: [
            "What makes this binary so large?",
            "Where does this app allocate memory?",
            "How much memory does this model need?",
            "Can this shader use fewer texture reads?",
        ],
    },
    Content {
        title: "Connect the pieces",
        prompts: [
            "Show me how these functions connect",
            "How does this app load its plugins?",
            "Explain this model's architecture",
            "How do these shader stages fit together?",
        ],
    },
    Content {
        title: "What is hiding inside?",
        prompts: [
            "Find the error paths in this binary",
            "How does this app handle a failed save?",
            "Does this model contain custom operators?",
            "What happens at this shader's edges?",
        ],
    },
];
