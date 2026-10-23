use super::research::Category;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SkillLibrary {
    pub path: PathBuf,
    pub repository: Option<String>,
    pub revision: Option<String>,
    pub enabled: bool,
    pub include_archives: bool,
    pub fallback_categories: Vec<Category>,
}

impl Default for SkillLibrary {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            repository: None,
            revision: None,
            enabled: true,
            include_archives: false,
            fallback_categories: Vec::new(),
        }
    }
}
