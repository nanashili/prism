use crate::debug::Action;
use crate::error::Result;
use crate::storage::verification::{EvidenceSource, SourceKind};
use crate::worker::State;
use serde_json::{Value, json};

impl State {
    pub(in crate::worker) fn execute_verified_debug(&mut self, action: Action) -> Result<String> {
        if let Some(project) = &self.debug_project {
            self.store.invalidate_runtime_evidence(project)?;
        }
        self.bind_debug_target(&action)?;
        let observation = observation_request(&action);

        let result = crate::debug::execute(
            &mut self.debugger,
            action,
            &self.settings,
            self.cancelled.clone(),
            self.sink.clone(),
        );
        let text = match result {
            Ok(text) => text,
            Err(error) => {
                self.debug_project = None;
                return Err(error);
            }
        };

        if let Some((command, args)) = observation {
            self.record_debug_observation(&command, &args, &text)?;
        }
        Ok(text)
    }

    fn record_debug_observation(&self, command: &str, args: &Value, text: &str) -> Result<()> {
        if let Some(project) = &self.debug_project
            && self
                .project
                .as_ref()
                .is_some_and(|open| open.target.sha256 == *project)
        {
            let reply: Value = serde_json::from_str(text)?;
            let key = serde_json::to_string(&("debugger", command, args))?;
            self.store.capture_evidence(
                project,
                &EvidenceSource {
                    key: &key,
                    subject: "__runtime__",
                    kind: SourceKind::Runtime,
                },
                &json!({"command":command,"arguments":args,"reply":reply}),
            )?;
        }
        Ok(())
    }

    fn bind_debug_target(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Launch { path, .. } => {
                self.debug_project = None;
                let target = crate::project::files::inspect(path, &self.cancelled)?;
                if self
                    .project
                    .as_ref()
                    .is_some_and(|project| project.target.sha256 == target.sha256)
                {
                    self.sync_verification_context()?;
                    self.debug_project = Some(target.sha256);
                }
            }
            // Attach does not establish that this process is the opened binary.
            Action::Attach(_) | Action::Stop => self.debug_project = None,
            Action::Request { .. } => {}
        }
        Ok(())
    }
}

fn observation_request(action: &Action) -> Option<(String, Value)> {
    match action {
        Action::Request { command, args }
            if matches!(
                command.as_str(),
                "evaluate" | "stackTrace" | "variables" | "scopes"
            ) =>
        {
            Some((command.clone(), args.clone()))
        }
        _ => None,
    }
}
