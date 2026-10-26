use super::{title, whole_file};
use crate::agent::PermissionDecision;
use crate::error::Result;
use serde_json::Value;

#[derive(Clone)]
pub(in crate::agent) struct Policy {
    permissions: crate::settings::permissions::Permissions,
    project: Option<String>,
    file: Option<std::path::PathBuf>,
    remember: bool,
    whole_file: bool,
}

impl Policy {
    pub fn new(prompt: &crate::agent::Prompt) -> Self {
        Self {
            permissions: prompt.settings.permissions.clone(),
            project: prompt.project.clone(),
            file: prompt.file.clone(),
            remember: prompt.settings.desktop.remember_answers,
            whole_file: prompt.settings.desktop.ask_before_whole_file,
        }
    }

    pub(super) fn response(&self, params: &Value) -> Result<Option<Value>> {
        use crate::agent::permission_policy;
        if permission_policy::level(params, &self.permissions)
            == crate::settings::permissions::Level::Never
        {
            return Ok(permission_policy::response(params, &self.permissions));
        }
        if self.whole_file && whole_file::reads(params, self.file.as_deref()) {
            return Ok(None);
        }
        if self.remember
            && let Some(project) = &self.project
            && crate::agent::remembered::allows(project, title(params))?
        {
            return Ok(permission_policy::select(params, PermissionDecision::Allow));
        }
        Ok(permission_policy::response(params, &self.permissions))
    }

    pub(super) fn save_answer(&self, params: &Value, response: &Value) -> Result<()> {
        let Some(project) = self.project.as_deref().filter(|_| self.remember) else {
            return Ok(());
        };
        let selected = response["outcome"]["optionId"].as_str();
        let always = params["options"].as_array().is_some_and(|options| {
            options.iter().any(|option| {
                selected.is_some()
                    && option["optionId"].as_str() == selected
                    && option["kind"] == "allow_always"
            })
        });
        if always {
            crate::agent::remembered::remember(project, title(params))?;
        }
        Ok(())
    }
}
