use super::{
    BATCH_INSTRUCTIONS, INVESTIGATION_INSTRUCTIONS, Mode, RESPONSE_INSTRUCTIONS, definitions,
    endpoint::Endpoint, transport,
};
use crate::error::Result;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

mod message;
mod pending;

use message::{error, reply, tool_result};

const VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub(super) struct Session {
    mode: Mode,
    endpoint: Option<Endpoint>,
    initialized: bool,
}

impl Session {
    pub(super) fn new(mode: Mode, endpoint: Option<Endpoint>) -> Self {
        Self {
            mode,
            endpoint,
            initialized: false,
        }
    }

    pub(super) fn run(
        &mut self,
        input: &mut impl BufRead,
        output: impl Write + Send + 'static,
    ) -> Result<()> {
        let mut pending = pending::Pending::new(output);
        let result = self.read_requests(input, &mut pending);
        pending.finish(result)
    }

    fn read_requests<W: Write + Send + 'static>(
        &mut self,
        input: &mut impl BufRead,
        pending: &mut pending::Pending<W>,
    ) -> Result<()> {
        while let Some(bytes) = transport::read(input)? {
            pending.reap()?;
            let mut request = if let Ok(request) = serde_json::from_slice::<Value>(&bytes) {
                request
            } else {
                pending.write(&error(Value::Null, -32700, "Invalid JSON"))?;
                continue;
            };
            let request_bytes = bytes.len();
            drop(bytes);
            if let Err(response) = message::validate(&request) {
                pending.write(&response)?;
                continue;
            }
            if pending.has_id(&request["id"]) {
                pending.write(&error(
                    request["id"].clone(),
                    -32600,
                    "Duplicate active request id",
                ))?;
                continue;
            }
            if self.initialized
                && pending.accept(
                    &mut request,
                    request_bytes,
                    self.mode,
                    self.endpoint.as_ref(),
                )?
            {
                continue;
            }
            if let Some(response) = self.respond(&request) {
                pending.write(&response)?;
            }
        }
        Ok(())
    }

    fn respond(&mut self, request: &Value) -> Option<Value> {
        let id = request.get("id")?.clone();
        let method = request["method"].as_str().unwrap_or_default();
        Some(match method {
            "initialize" => self.initialize(id, &request["params"]),
            "ping" => reply(id, json!({})),
            _ if !self.initialized => error(id, -32000, "Initialize the server first"),
            "tools/list" => reply(id, definitions(self.mode)),
            _ => error(id, -32601, "Method not found"),
        })
    }

    fn initialize(&mut self, id: Value, params: &Value) -> Value {
        if self.initialized {
            return error(id, -32600, "The server is already initialized");
        }
        let Some(requested) = params["protocolVersion"].as_str() else {
            return error(id, -32602, "A protocolVersion is required");
        };
        let version = if VERSIONS.contains(&requested) {
            requested
        } else {
            VERSIONS[0]
        };
        self.initialized = true;
        let mut instructions = if self.mode == Mode::Headless {
            format!(
                "This is an unattended local Prism worker. UI navigation and user questions are unavailable. Analysis and reconstruction tools operate on the project opened for this process. Analysis results may be incomplete.\n\n{BATCH_INSTRUCTIONS}\n\n{RESPONSE_INSTRUCTIONS}"
            )
        } else {
            format!(
                "Tools use the open Prism window and its existing analysis worker. Open Prism before calling tools. When available, ask questions with request_user_input and wait for the user's answer. Skip and dismissal are never approval. Analysis results may be incomplete.\n\n{BATCH_INSTRUCTIONS}\n\n{RESPONSE_INSTRUCTIONS}"
            )
        };
        instructions.push_str("\n\n");
        instructions.push_str(INVESTIGATION_INSTRUCTIONS);
        reply(
            id,
            json!({
                "protocolVersion":version,"capabilities":{"tools":{}},
                "serverInfo":{"name":self.mode.name(),"title":"Prism","version":env!("CARGO_PKG_VERSION")},
                "instructions":instructions
            }),
        )
    }
}
