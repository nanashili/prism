use crate::model::AgentUpdate;
use crate::ui::app::Prism;
use crate::ui::notifications::Level;
use crate::ui::presentation::SessionAge;

impl Prism {
    pub(in crate::ui) fn receive_agent_update(&mut self, update: AgentUpdate) {
        match update {
            AgentUpdate::OptionRejected { id, value, reason } => {
                self.reject_option(&id, &value, &reason);
            }
            AgentUpdate::Commands(commands) => self.chat.commands = commands,
            AgentUpdate::SessionClosed => self.chat.connected = false,
            AgentUpdate::Goal(ref goal) => {
                self.chat.goal.clone_from(goal);
                self.record_turn_update(update);
            }
            AgentUpdate::Usage(ref usage) => {
                self.chat.usage = Some(usage.clone());
                self.record_turn_update(update);
            }
            AgentUpdate::Notice(text) => {
                self.notify(Level::Info, "Agent notice", text.trim(), "");
            }
            update => self.record_turn_update(update),
        }
    }

    pub(in crate::ui) fn pick_option(
        &mut self,
        menu: &crate::agent::options::OptionMenu,
        value: String,
    ) {
        let pick = crate::agent::options::Pick {
            id: if menu.category == "model" {
                crate::agent::options::MODEL_PICK.into()
            } else {
                menu.id.clone()
            },
            value,
            modes_api: menu.modes_api,
        };
        self.apply_agent_pick(pick);
    }

    pub(in crate::ui) fn apply_agent_pick(&mut self, pick: crate::agent::options::Pick) {
        if let Some(agent) = &self.agent
            && let Err(error) = agent.set_option(pick.clone())
        {
            self.report_error(error);
            return;
        }
        let key = crate::agent::picks::key(&self.settings);
        if pick.id != crate::agent::options::MODEL_PICK
            && let Err(error) = crate::agent::picks::remember(&key, &pick.id, Some(&pick.value))
        {
            self.report_error(error);
        }
        let menus = crate::agent::options::menus(&self.chat.options);
        self.chat.picks.retain(|previous| {
            previous.id != pick.id
                && !menus.iter().any(|menu| {
                    menu.category == "model" && pick.matches(menu) && previous.matches(menu)
                })
        });
        self.chat.picks.push(pick);
        self.chat.composer_revision += 1;
    }

    pub(in crate::ui) fn receive_agent_options(&mut self, options: serde_json::Value) {
        let new_session = options.get("sessionId").is_some();
        if new_session {
            self.chat.usage = None;
            self.chat.goal = None;
            self.chat.connected = true;
        }
        crate::agent::options::merge(&mut self.chat.options, options);
        let choices = crate::agent::models::choices(&self.chat.options);
        if !choices.is_empty() {
            self.agent_catalog
                .models
                .insert(crate::agent::picks::key(&self.settings), choices);
            self.agent_catalog.models_revision += 1;
        }
        let menus = crate::agent::options::menus(&self.chat.options);
        if let Err(error) = self.remember_confirmed_model(&menus, SessionAge::from(new_session)) {
            self.report_error(error);
        }
        self.chat.picks.retain(|pick| {
            !menus
                .iter()
                .any(|menu| pick.matches(menu) && menu.current == pick.value)
        });
        self.chat.composer_revision += 1;
        if self.chat.busy {
            self.chat.status = "Working".into();
        }
    }

    fn reject_option(&mut self, id: &str, value: &str, reason: &str) {
        self.chat
            .picks
            .retain(|pick| pick.id != id || pick.value != value);
        self.chat.composer_revision += 1;
        let menus = crate::agent::options::menus(&self.chat.options);
        let previous = menus
            .iter()
            .find(|menu| {
                menu.id == id
                    || (id == crate::agent::options::MODEL_PICK && menu.category == "model")
            })
            .map(|menu| menu.current.as_str());
        let key = crate::agent::picks::key(&self.settings);
        if id == crate::agent::options::MODEL_PICK
            && self.chat.models.get(&key).map(String::as_str) == Some(value)
            && let Err(error) = self.remember_chat_model(previous)
        {
            self.report_error(error);
        }
        if let Err(error) = crate::agent::picks::restore_if_current(&key, id, value, previous) {
            self.report_error(error);
        }
        self.report_error(crate::error::failure(format!(
            "Couldn't change the agent option: {reason}"
        )));
    }

    fn record_turn_update(&mut self, update: AgentUpdate) {
        let persist_goal = matches!(update, AgentUpdate::Goal(_));
        let Some(message) = self.chat.messages.last_mut() else {
            return;
        };
        let Some(activity) = &mut message.activity else {
            return;
        };
        update.apply_to_turn(&mut activity.progress);
        activity.retain_recent();

        if persist_goal
            && activity.ended.is_some()
            && let Some(id) = message.id
            && let Err(error) = self.store.update_message_activity(id, activity)
        {
            self.report_error(error);
        }
    }
}
