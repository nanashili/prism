use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod analysis_backend;
pub(crate) mod chat_appearance;
mod conversation;
mod desktop;
pub(crate) mod permissions;
pub(crate) mod research;
pub(crate) mod skill_library;
mod tools;
mod views;

#[cfg(test)]
mod tests;

pub(crate) use analysis_backend::AnalysisBackend;
pub(crate) use tools::Tools;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub research: research::Research,
    pub desktop: desktop::Desktop,
    pub conversation: conversation::Conversation,
    pub views: views::Views,
    pub tools: Tools,
    pub appearance: String,
    pub code_size: f32,
    pub code_font: String,
    pub color_scheme: String,
    pub line_spacing: String,
    pub line_numbers: bool,
    pub graph_uses_code_size: bool,
    pub transparency: f32,
    pub ui_scale: f32,
    pub provider: String,
    pub rea: String,
    pub rizin: String,
    pub agent: String,
    pub agent_args: String,
    pub agent_id: String,
    pub agent_name: String,
    pub permissions: permissions::Permissions,
    pub lldb: String,
    pub jadx: String,
    pub cache_mib: u64,
    pub artifact_cache_mib: u64,
    pub analysis_memory_mib: u32,
    pub timeout_seconds: u64,
    pub recent_limit: usize,
}

impl Default for Settings {
    fn default() -> Self {
        let (agent, agent_args) = crate::executable_paths::codex_agent();
        Self {
            research: research::Research::default(),
            desktop: Default::default(),
            conversation: Default::default(),
            views: Default::default(),
            tools: Default::default(),
            appearance: "System".into(),
            code_size: 10.0,
            code_font: "JetBrains Mono".into(),
            color_scheme: "Prism".into(),
            line_spacing: "Normal".into(),
            line_numbers: true,
            graph_uses_code_size: true,
            transparency: if cfg!(target_os = "linux") { 0.0 } else { 30.0 },
            ui_scale: 1.0,
            provider: AnalysisBackend::default_provider().into(),
            rea: std::env::var("PRISM_REA").unwrap_or_else(|_| "rea".into()),
            rizin: std::env::var("REA_RIZIN_COMMAND")
                .unwrap_or_else(|_| crate::executable_paths::find("rizin")),
            agent,
            agent_args,
            agent_id: String::new(),
            agent_name: String::new(),
            permissions: permissions::Permissions::default(),
            lldb: std::env::var("PRISM_LLDB_DAP")
                .unwrap_or_else(|_| crate::executable_paths::find("lldb-dap")),
            jadx: std::env::var("PRISM_JADX")
                .unwrap_or_else(|_| crate::executable_paths::find("jadx")),
            cache_mib: 256,
            artifact_cache_mib: 1024,
            analysis_memory_mib: 1024,
            timeout_seconds: 300,
            recent_limit: 30,
        }
    }
}

pub(crate) fn support_dir() -> PathBuf {
    prism::mcp::support_dir()
}

impl Settings {
    pub(crate) fn analysis_backend(&self) -> AnalysisBackend {
        AnalysisBackend::from_provider(&self.provider)
    }

    pub fn load() -> Result<Self> {
        let path = support_dir().join("settings.json");
        match std::fs::read(path) {
            Ok(bytes) => {
                let settings: Self = serde_json::from_slice(&bytes)?;
                settings.validate()?;
                Ok(settings)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self) -> Result<()> {
        self.validate()?;
        crate::storage::atomic::write(&support_dir().join("settings.json"), |file| {
            serde_json::to_writer_pretty(file, self)?;
            Ok(())
        })
    }

    pub fn same_analysis_backend(&self, other: &Self) -> bool {
        let backend = self.analysis_backend();
        self.provider == other.provider
            && self.rea == other.rea
            && self.rizin == other.rizin
            && self.jadx == other.jadx
            && self.analysis_memory_mib == other.analysis_memory_mib
            && self.timeout_seconds == other.timeout_seconds
            && self.tools.same_analysis(&other.tools, backend)
    }

    fn validate(&self) -> Result<()> {
        self.analysis_backend().require_available()?;
        self.conversation.appearance.validate()?;

        if !(1..=8).contains(&self.tools.ghidra_threads) {
            return Err("Ghidra decompiler workers must be between 1 and 8".into());
        }

        let hidden = &self.conversation.hidden_agents;
        if hidden.len() > 128 || hidden.iter().any(|id| id.is_empty() || id.len() > 4096) {
            return Err("Hidden agents exceed their supported limits".into());
        }

        if !(400.0..=760.0).contains(&self.desktop.chat_panel_width) {
            return Err("The conversation panel width is outside its supported range".into());
        }
        if !(9.0..=20.0).contains(&self.code_size) || !(0.8..=1.5).contains(&self.ui_scale) {
            return Err("Font size or interface scale is outside its supported range".into());
        }
        if !(0.0..=100.0).contains(&self.transparency)
            || !["Prism", "Classic", "Quiet"].contains(&self.color_scheme.as_str())
            || !["Compact", "Normal", "Relaxed"].contains(&self.line_spacing.as_str())
        {
            return Err("A display setting is outside its supported range".into());
        }
        if !(16..=2048).contains(&self.cache_mib)
            || !(256..=16384).contains(&self.artifact_cache_mib)
            || !(256..=8192).contains(&self.analysis_memory_mib)
            || !(30..=1800).contains(&self.timeout_seconds)
            || !(1..=100).contains(&self.recent_limit)
        {
            return Err("A resource limit in settings.json is outside its supported range".into());
        }
        Ok(())
    }
}
