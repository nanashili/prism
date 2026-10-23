use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Level {
    Allow,
    Ask,
    Never,
}

impl Level {
    pub const ALL: [Self; 3] = [Self::Allow, Self::Ask, Self::Never];

    pub fn label(self) -> &'static str {
        match self {
            Self::Allow => "Allow",
            Self::Ask => "Ask",
            Self::Never => "Never",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.label() == label)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Tool {
    Read,
    View,
    Rename,
    Notes,
    Patch,
    Gpu,
    Debug,
    Shell,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Permissions {
    pub read: Level,
    pub view: Level,
    pub rename: Level,
    pub notes: Level,
    pub patch: Level,
    pub gpu: Level,
    pub debug: Level,
    pub shell: Level,
}

impl Default for Permissions {
    fn default() -> Self {
        Self::preset("Balanced")
    }
}

impl Permissions {
    pub fn preset(name: &str) -> Self {
        use Level::{Allow, Ask};
        let edits = if name == "Free" { Allow } else { Ask };
        Self {
            read: if name == "Careful" { Ask } else { Allow },
            view: Allow,
            rename: edits,
            notes: edits,
            patch: Ask,
            gpu: edits,
            debug: edits,
            shell: Ask,
        }
    }

    pub fn level(&self, tool: Tool) -> Level {
        match tool {
            Tool::Read => self.read,
            Tool::View => self.view,
            Tool::Rename => self.rename,
            Tool::Notes => self.notes,
            Tool::Patch => self.patch,
            Tool::Gpu => self.gpu,
            Tool::Debug => self.debug,
            Tool::Shell => self.shell,
        }
    }

    pub fn set(&mut self, tool: Tool, level: Level) {
        match tool {
            Tool::Read => self.read = level,
            Tool::View => self.view = level,
            Tool::Rename => self.rename = level,
            Tool::Notes => self.notes = level,
            Tool::Patch => self.patch = level,
            Tool::Gpu => self.gpu = level,
            Tool::Debug => self.debug = level,
            Tool::Shell => self.shell = level,
        }
    }
}
