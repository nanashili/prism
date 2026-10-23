use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Category {
    Binaries,
    Firmware,
    Web,
    Mobile,
    Network,
    GameEngines,
    GameRuntime,
    GameSecurity,
    Graphics,
}

impl Category {
    pub const ALL: [Self; 9] = [
        Self::Binaries,
        Self::Firmware,
        Self::Web,
        Self::Mobile,
        Self::Network,
        Self::GameEngines,
        Self::GameRuntime,
        Self::GameSecurity,
        Self::Graphics,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Binaries => "Binaries and desktop apps",
            Self::Firmware => "Firmware and hardware",
            Self::Web => "Web applications",
            Self::Mobile => "Mobile apps",
            Self::Network => "Networks and protocols",
            Self::GameEngines => "Games: engines and assets",
            Self::GameRuntime => "Games: runtime and memory",
            Self::GameSecurity => "Games: security research",
            Self::Graphics => "Graphics and models",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Binaries => "Native executables, libraries, disassembly and reconstruction.",
            Self::Firmware => "Embedded images, boot firmware, drivers and device interfaces.",
            Self::Web => "JavaScript, browser behavior and web APIs through your agent and skills.",
            Self::Mobile => "Android packages, iOS apps and mobile runtimes.",
            Self::Network => {
                "Protocol analysis, traffic and client/server behavior through your agent."
            }
            Self::GameEngines => "Unity, Unreal, Source, object layouts and asset formats.",
            Self::GameRuntime => "Debugging, memory layouts and runtime instrumentation.",
            Self::GameSecurity => "Anti-cheat architecture and game security analysis.",
            Self::Graphics => "Shaders, rendering pipelines and machine learning models.",
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            Self::Binaries => "Help me understand this binary's entry point",
            Self::Firmware => "Help me identify the sections in this firmware image",
            Self::Web => "Help me trace the behavior of this web application",
            Self::Mobile => "Help me understand how this mobile app starts",
            Self::Network => "Help me map this application's network protocol",
            Self::GameEngines => "Help me identify this game's engine and asset formats",
            Self::GameRuntime => "Help me understand this game's runtime data structures",
            Self::GameSecurity => "Help me review this game's security architecture",
            Self::Graphics => "Help me understand this shader or model",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Research {
    pub categories: Vec<Category>,
    pub setup_complete: bool,
}

impl Default for Research {
    fn default() -> Self {
        Self {
            categories: vec![Category::Binaries],
            setup_complete: false,
        }
    }
}

impl Research {
    pub fn legacy() -> Self {
        Self {
            categories: Category::ALL.to_vec(),
            setup_complete: true,
        }
    }

    pub fn contains(&self, category: Category) -> bool {
        self.categories.contains(&category)
    }

    pub fn matches(&self, categories: &[Category]) -> bool {
        categories.is_empty() || categories.iter().any(|category| self.contains(*category))
    }

    pub fn set(&mut self, category: Category, enabled: bool) {
        self.categories.retain(|entry| *entry != category);
        if enabled {
            self.categories.push(category);
        }
    }

    pub fn native_tools(&self) -> bool {
        self.categories
            .iter()
            .any(|category| !matches!(category, Category::Web | Category::Network))
    }

    pub fn debugger(&self) -> bool {
        self.matches(&[
            Category::Binaries,
            Category::Mobile,
            Category::GameRuntime,
            Category::GameSecurity,
        ])
    }
}
