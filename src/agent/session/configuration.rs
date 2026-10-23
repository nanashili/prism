use super::{Session, control};
use crate::agent::{Prompt, options, picks};
use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::model::AgentUpdate;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Duration;

impl Session {
    pub(super) fn configure(
        &mut self,
        prompt: &Prompt,
        restore: bool,
        sink: &Sink,
        cancelled: &AtomicBool,
        controls: &Receiver<super::Control>,
    ) -> Result<()> {
        let mut choices = choices(prompt, restore, sink)?;
        let menus = options::menus(&self.updates.options);
        select_model(&mut choices, &menus, prompt, restore, sink)?;
        choices.sort_by_key(|pick| {
            pick.id != options::MODEL_PICK
                && !menus
                    .iter()
                    .any(|menu| menu.id == pick.id && menu.category == "model")
        });

        for pick in choices {
            if cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            let Some(resolved) = options::resolve(&self.updates.options, &pick) else {
                if pick.id == options::MODEL_PICK {
                    sink.send(Event::AgentUpdate(AgentUpdate::OptionRejected {
                        id: pick.id,
                        value: pick.value,
                        reason: "This agent no longer offers that model".into(),
                    }))?;
                }
                continue;
            };
            if options::needs_change(&self.updates.options, &resolved) {
                self.configure_option(pick, resolved, sink, cancelled, controls)?;
            }
        }
        Ok(())
    }

    fn configure_option(
        &mut self,
        pick: options::Pick,
        resolved: options::Pick,
        sink: &Sink,
        cancelled: &AtomicBool,
        controls: &Receiver<super::Control>,
    ) -> Result<()> {
        let (method, params) = control::option_request(&self.id, &resolved);
        let result = self.call(
            method,
            params,
            Duration::from_secs(30),
            sink,
            cancelled,
            controls,
        );
        let response = match result {
            Ok(response) => response,
            Err(error) => {
                if !self.rpc.is_reusable() {
                    return Err(error);
                }
                return sink.send(Event::AgentUpdate(AgentUpdate::OptionRejected {
                    id: pick.id,
                    value: pick.value,
                    reason: error.to_string(),
                }));
            }
        };
        let response = if resolved.legacy {
            serde_json::json!({"modes":{"currentModeId":resolved.value}})
        } else {
            response
        };
        self.updates.publish_options(response, sink)
    }
}

fn select_model(
    choices: &mut Vec<options::Pick>,
    menus: &[options::OptionMenu],
    prompt: &Prompt,
    restore: bool,
    sink: &Sink,
) -> Result<()> {
    let Some(model) = menus.iter().find(|menu| menu.category == "model") else {
        return Ok(());
    };
    let requested = prompt.picks.iter().find(|pick| pick.matches(model));
    let available = requested.filter(|pick| {
        model
            .choices
            .iter()
            .any(|choice| choice.value == pick.value)
    });
    if let Some(rejected) = requested.filter(|_| available.is_none()) {
        sink.send(Event::AgentUpdate(AgentUpdate::OptionRejected {
            id: rejected.id.clone(),
            value: rejected.value.clone(),
            reason: "This agent no longer offers that model. Using its first available model."
                .into(),
        }))?;
    }

    // Global model preferences must not override the choice belonging to this chat.
    choices.retain(|pick| !pick.matches(model));
    let selected = match available {
        Some(pick) => Some(&pick.value),
        None if restore || requested.is_some() => model.choices.first().map(|choice| &choice.value),
        None => None,
    };
    if let Some(value) = selected {
        choices.push(options::Pick {
            id: options::MODEL_PICK.into(),
            value: value.clone(),
            legacy: model.legacy,
        });
    }
    Ok(())
}

fn choices(prompt: &Prompt, restore: bool, sink: &Sink) -> Result<Vec<options::Pick>> {
    let mut choices = if restore {
        match picks::load(&picks::key(&prompt.settings)) {
            Ok(choices) => choices,
            Err(error) => {
                sink.send(Event::AgentUpdate(AgentUpdate::Notice(format!(
                    "Couldn't restore agent choices: {error}"
                ))))?;
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    for pick in &prompt.picks {
        choices.retain(|saved| saved.id != pick.id);
        choices.push(pick.clone());
    }
    Ok(choices)
}
