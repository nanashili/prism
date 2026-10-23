mod pending;
mod policy;
mod request;
mod whole_file;

pub(super) use pending::Requests;
pub(super) use policy::Policy;

use crate::error::Result;
use crate::events::{Event, Sink};
use serde_json::Value;

fn title(params: &Value) -> &str {
    params["toolCall"]["title"].as_str().unwrap_or("Use a tool")
}

fn notice(sink: &Sink, context: &str, error: impl std::fmt::Display) -> Result<()> {
    sink.send(Event::AgentUpdate(crate::model::AgentUpdate::Notice(
        format!("{context}: {error}"),
    )))
}
