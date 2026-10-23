use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Desktop {
    pub launch: String,
    pub intro_seen: bool,
    pub restore_layout: bool,
    pub chat_panel_width: f32,
    pub auto_update: bool,
    pub update_channel: String,
    pub show_whats_new: bool,
    pub show_suggestions: bool,
    pub frost: f32,
    pub remember_answers: bool,
    pub hide_paths: bool,
    pub ask_before_whole_file: bool,
    pub count_install: bool,
    pub crash_reports: bool,
    pub usage_stats: bool,
    pub disabled_skills: Vec<String>,
    pub skills_in_composer: bool,
    pub skills_dir: String,
    pub skill_libraries: Vec<super::skill_library::SkillLibrary>,
    pub export_dir: String,
    pub reveal_exports: bool,
    pub autosave: bool,
}

impl Default for Desktop {
    fn default() -> Self {
        Self {
            launch: "Home".into(),
            intro_seen: false,
            restore_layout: true,
            chat_panel_width: 480.0,
            auto_update: false,
            update_channel: "Stable".into(),
            show_whats_new: true,
            show_suggestions: true,
            frost: 60.0,
            remember_answers: true,
            hide_paths: true,
            ask_before_whole_file: false,
            count_install: false,
            crash_reports: false,
            usage_stats: false,
            disabled_skills: Vec::new(),
            skills_in_composer: true,
            skills_dir: String::new(),
            skill_libraries: Vec::new(),
            export_dir: String::new(),
            reveal_exports: true,
            autosave: true,
        }
    }
}
