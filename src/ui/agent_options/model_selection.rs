use crate::agent::options::{MODEL_PICK, OptionMenu, Pick};
use crate::error::Result;
use crate::ui::app::Prism;

impl Prism {
    pub(super) fn cached_model_menu(&self) -> Option<OptionMenu> {
        let key = crate::agent::picks::key(&self.settings);
        let choices = self.agent_catalog.models.get(&key).or_else(|| {
            if !self.settings.agent_id.is_empty() {
                return None;
            }
            self.agent_catalog
                .models
                .get(crate::ui::agent_picker::custom_id(&self.settings))
        })?;
        let first = choices.first()?;
        Some(OptionMenu {
            id: MODEL_PICK.into(),
            name: "Model".into(),
            current: first.value.clone(),
            category: "model".into(),
            choices: choices.clone(),
            legacy: false,
        })
    }

    pub(in crate::ui) fn prompt_option_picks(&self) -> Vec<Pick> {
        let mut picks = self.chat.picks.clone();
        let menus = self.composer_option_menus();
        if let Some(model) = menus.iter().find(|menu| menu.category == "model") {
            picks.retain(|pick| !pick.matches(model));
            picks.push(Pick {
                id: MODEL_PICK.into(),
                value: model.current.clone(),
                legacy: model.legacy,
            });
        } else if !picks.iter().any(|pick| pick.id == MODEL_PICK)
            && let Some(value) = self
                .chat
                .models
                .get(&crate::agent::picks::key(&self.settings))
        {
            picks.push(Pick {
                id: MODEL_PICK.into(),
                value: value.clone(),
                legacy: false,
            });
        }
        picks
    }

    pub(in crate::ui) fn remember_selected_model(&mut self) -> Result<()> {
        let menus = self.composer_option_menus();
        if let Some(model) = menus.iter().find(|menu| menu.category == "model") {
            self.remember_chat_model(Some(&model.current))?;
        }
        Ok(())
    }

    pub(in crate::ui) fn remember_confirmed_model(
        &mut self,
        menus: &[OptionMenu],
        new_session: bool,
    ) -> Result<()> {
        let Some(model) = menus.iter().find(|menu| menu.category == "model") else {
            return Ok(());
        };
        let pending = self.chat.picks.iter().find(|pick| pick.matches(model));
        let requested = pending.map(|pick| pick.value.as_str());
        let confirmed = !new_session || requested == Some(model.current.as_str());
        let superseded = requested.is_some_and(|value| value != model.current);
        let available = model
            .choices
            .iter()
            .any(|choice| choice.value == model.current);
        if !confirmed || superseded || !available {
            return Ok(());
        }

        self.remember_chat_model(Some(&model.current))
    }

    pub(in crate::ui) fn remember_chat_model(&mut self, value: Option<&str>) -> Result<()> {
        let key = crate::agent::picks::key(&self.settings);
        if self.chat.models.get(&key).map(String::as_str) == value {
            return Ok(());
        }
        let previous = match value {
            Some(value) => self.chat.models.insert(key.clone(), value.into()),
            None => self.chat.models.remove(&key),
        };
        if let Some(chat) = self.chat.id
            && let Err(error) = self.store.save_chat_models(chat, &self.chat.models)
        {
            match previous {
                Some(previous) => {
                    self.chat.models.insert(key, previous);
                }
                None => {
                    self.chat.models.remove(&key);
                }
            }
            return Err(error);
        }
        self.chat.composer_revision += 1;
        Ok(())
    }
}
