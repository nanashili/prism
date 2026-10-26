use super::Session;
use crate::agent::options::Pick;
use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::model::AgentUpdate;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub(crate) enum Control {
    Option(Pick),
    Goal(&'static str),
}

pub(super) struct Pending {
    control: Control,
    started: Instant,
}

pub(super) type Requests = BTreeMap<u64, Pending>;

impl Session {
    pub(super) fn send_controls(
        &mut self,
        receiver: &Receiver<Control>,
        sink: &Sink,
    ) -> Result<()> {
        let waiting: Vec<_> = self.pending.keys().copied().collect();
        for id in waiting {
            if let Some(mut message) = self.rpc.take_reply(id) {
                self.control_reply(&mut message, sink)?;
            }
        }

        while self.pending.len() < 8 {
            let Ok(control) = receiver.try_recv() else {
                break;
            };
            self.begin_control(control, sink)?;
        }

        let expired: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.started.elapsed() > Duration::from_secs(30))
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            if let Some(pending) = self.pending.remove(&id) {
                self.control_failed(
                    pending.control,
                    "The agent did not acknowledge the change",
                    sink,
                )?;
            }
        }
        Ok(())
    }

    fn begin_control(&mut self, control: Control, sink: &Sink) -> Result<()> {
        let (method, params) = match &control {
            Control::Option(pick) => {
                let Some(resolved) = crate::agent::options::resolve(&self.updates.options, pick)
                else {
                    return self.control_failed(
                        control,
                        "This agent no longer offers that option",
                        sink,
                    );
                };
                if !crate::agent::options::needs_change(&self.updates.options, &resolved) {
                    return Ok(());
                }
                option_request(&self.id, &resolved)
            }
            Control::Goal(action) => (
                "_session/goal",
                json!({"sessionId":self.id,"action":action}),
            ),
        };
        let id = match self.rpc.begin(method, params) {
            Ok(id) => id,
            Err(error) => return self.control_failed(control, &error.to_string(), sink),
        };
        self.pending.insert(
            id,
            Pending {
                control,
                started: Instant::now(),
            },
        );
        Ok(())
    }

    pub(super) fn control_reply(&mut self, message: &mut Value, sink: &Sink) -> Result<bool> {
        let Some(id) = crate::protocol::response_id(message) else {
            return Ok(false);
        };
        let Some(pending) = self.pending.remove(&id) else {
            return Ok(false);
        };
        match crate::protocol::response_result(message.take()) {
            Ok(mut response) => {
                if let Control::Option(pick) = pending.control {
                    if pick.modes_api {
                        response = json!({"modes":{"currentModeId":pick.value}});
                    }
                    self.updates.publish_options(response, sink)?;
                }
            }
            Err(error) => self.control_failed(pending.control, &error.to_string(), sink)?,
        }
        Ok(true)
    }

    fn control_failed(&self, control: Control, reason: &str, sink: &Sink) -> Result<()> {
        let update = match control {
            Control::Option(pick) => AgentUpdate::OptionRejected {
                id: pick.id,
                value: pick.value,
                reason: reason.into(),
            },
            Control::Goal(_) => AgentUpdate::Notice(format!("Couldn't update the goal: {reason}")),
        };
        sink.send(Event::AgentUpdate(update))
    }
}

pub(super) fn option_request(id: &str, pick: &Pick) -> (&'static str, Value) {
    if pick.modes_api {
        (
            "session/set_mode",
            json!({"sessionId":id,"modeId":pick.value}),
        )
    } else {
        (
            "session/set_config_option",
            json!({"sessionId":id,"configId":pick.id,"value":pick.value}),
        )
    }
}

pub(in crate::agent) fn unavailable(control: Control, sink: &Sink) -> Result<()> {
    if let Control::Goal(_) = control {
        return sink.send(Event::AgentUpdate(AgentUpdate::Notice(
            failure("The agent session has closed. Start a new turn first.").to_string(),
        )));
    }
    // Option picks are already saved by the UI and restored on the next session.
    Ok(())
}
