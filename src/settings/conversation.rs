use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Conversation {
    pub appearance: super::chat_appearance::ChatAppearance,
    pub show_thinking: bool,
    pub history: String,
    pub open_workbench: String,
    pub send_key: String,
    pub notify_when_done: bool,
    pub notify_when_asked: bool,
    pub notify_when_ready: bool,
    pub sound_when_asked: bool,
    pub keep_agents_running: bool,
    pub start_agent_with_project: bool,
    pub prism_tools: bool,
    pub rea_tools: bool,
    pub ida_tools: bool,
    pub ask_model: String,
    pub ask_fast: bool,
    pub hidden_agents: Vec<String>,
    pub custom_agent: String,
    pub custom_tool_server: String,
}

impl Default for Conversation {
    fn default() -> Self {
        Self {
            appearance: Default::default(),
            show_thinking: true,
            history: "Normal".into(),
            open_workbench: "When the agent opens code".into(),
            send_key: "Return".into(),
            notify_when_done: false,
            notify_when_asked: false,
            notify_when_ready: false,
            sound_when_asked: false,
            keep_agents_running: true,
            start_agent_with_project: false,
            prism_tools: true,
            rea_tools: true,
            ida_tools: false,
            ask_model: "Same as the chat".into(),
            ask_fast: false,
            hidden_agents: Vec::new(),
            custom_agent: String::new(),
            custom_tool_server: String::new(),
        }
    }
}
