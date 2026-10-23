use crate::settings::permissions::{Level, Tool};
use eframe::egui;

#[derive(Clone)]
pub(in crate::ui) enum Command {
    Engine(String),
    Reset,
    ClearChats,
    ClearCache,
    ClearRecordings,
    ForgetAnswers,
    ReviewChats(bool),
    StorageOverview,
    DeleteStoredChat(i64),
    RestoreStoredChat(i64),
    ResetLayout,
    Setup,
    CopyDetails,
    CheckUpdates,
    Open(&'static str),
    Reveal(std::path::PathBuf),
    Choose(Path),
    ResetPath(Path),
    Agent(String, String),
    BrowseAgents,
    AgentVisibility(String),
    Rescan,
    CustomAgent,
    CustomToolServer,
    NewSkill,
    Skill(String, bool),
    Permission(Tool, Level),
}

#[derive(Clone, Copy)]
pub(in crate::ui) enum Path {
    Exports,
    Skills,
    Debugger,
    Rizin,
    Rea,
    #[cfg(feature = "prism-ghidra-rea")]
    Ghidra,
    #[cfg(feature = "prism-ghidra-rea")]
    Java,
    ReaNode,
    #[cfg(feature = "rea-rust-ghidra")]
    RustRea,
    #[cfg(feature = "rea-rust-ghidra")]
    RustGhidraWorker,
    Hopper,
    Ida,
    Jadx,
}

pub(in crate::ui) enum Control<'a> {
    None,
    Toggle(&'a mut bool),
    Choice(&'a mut String, &'static [&'static str], bool),
    Memory(&'a mut u32),
    Timeout(&'a mut u64),
    #[cfg(feature = "prism-ghidra-rea")]
    Workers(&'a mut u8),
    Range(&'a mut f32, f32, f32, f32, &'static str),
    Text(&'a mut String, &'static str),
    Permission(Tool, Level),
    Skill(String, bool),
}

pub(in crate::ui) enum Lead {
    Agent(String),
    Image(egui::ImageSource<'static>),
}

pub(in crate::ui) struct Button {
    pub label: String,
    pub command: Command,
    pub danger: bool,
    pub primary: bool,
    pub unavailable: Option<&'static str>,
}

impl Button {
    pub fn new(label: impl Into<String>, command: Command) -> Self {
        Self {
            label: label.into(),
            command,
            danger: false,
            primary: false,
            unavailable: None,
        }
    }
}

pub(in crate::ui) struct Row<'a> {
    pub title: String,
    pub description: String,
    pub control: Control<'a>,
    pub buttons: Vec<Button>,
    pub lead: Option<Lead>,
    pub tag: Option<&'static str>,
    pub status: Option<(String, bool)>,
    pub unavailable: Option<&'static str>,
    pub mono: bool,
    pub pick: Option<(String, bool)>,
}

impl<'a> Row<'a> {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            control: Control::None,
            buttons: Vec::new(),
            lead: None,
            tag: None,
            status: None,
            unavailable: None,
            mono: false,
            pick: None,
        }
    }

    pub fn toggle(mut self, value: &'a mut bool) -> Self {
        self.control = Control::Toggle(value);
        self
    }

    pub fn select(mut self, value: &'a mut String, options: &'static [&'static str]) -> Self {
        self.control = Control::Choice(value, options, false);
        self
    }

    pub fn segmented(mut self, value: &'a mut String, options: &'static [&'static str]) -> Self {
        self.control = Control::Choice(value, options, true);
        self
    }

    pub fn range(
        mut self,
        value: &'a mut f32,
        min: f32,
        max: f32,
        step: f32,
        unit: &'static str,
    ) -> Self {
        self.control = Control::Range(value, min, max, step, unit);
        self
    }

    pub fn input(mut self, value: &'a mut String, hint: &'static str) -> Self {
        self.control = Control::Text(value, hint);
        self
    }

    pub fn button(mut self, label: impl Into<String>, command: Command) -> Self {
        self.buttons.push(Button::new(label, command));
        self
    }

    pub fn blocked(mut self, reason: &'static str) -> Self {
        self.unavailable = Some(reason);
        self
    }

    pub fn image(mut self, source: egui::ImageSource<'static>) -> Self {
        self.lead = Some(Lead::Image(source));
        self
    }

    pub fn status(mut self, label: impl Into<String>, good: bool) -> Self {
        self.status = Some((label.into(), good));
        self
    }
}

pub(in crate::ui) struct Group<'a> {
    pub label: &'static str,
    pub rows: Vec<Row<'a>>,
    pub foot: &'static str,
    pub preview: Option<Preview>,
}

impl<'a> Group<'a> {
    pub fn new(label: &'static str, rows: Vec<Row<'a>>) -> Self {
        Self {
            label,
            rows,
            foot: "",
            preview: None,
        }
    }

    pub fn foot(mut self, text: &'static str) -> Self {
        self.foot = text;
        self
    }
}

pub(in crate::ui) enum Preview {
    Address(crate::settings::Settings),
    Chat(crate::settings::Settings),
}
