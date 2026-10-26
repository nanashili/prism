use super::State;
use crate::analysis::execution::{ExecutionTrace, MAX_TRACE_BYTES, ModuleMapping, TraceSource};
use crate::error::{Result, failure};
use crate::model::View;
use serde_json::{Value, json};

impl State {
    pub(super) fn execute_evidence_debug(
        &mut self,
        action: crate::debug::Action,
    ) -> Result<String> {
        let project = self.debug_launch_identity(&action)?;
        let text = crate::debug::execute(
            &mut self.debugger,
            action,
            &self.settings,
            self.cancelled.clone(),
            self.sink.clone(),
        )?;
        if let Some(project) = project
            && let Some(debugger) = &mut self.debugger
        {
            debugger.bind_project(&project);
        }
        Ok(text)
    }

    fn debug_launch_identity(&self, action: &crate::debug::Action) -> Result<Option<String>> {
        let crate::debug::Action::Launch { path, .. } = action else {
            return Ok(None);
        };
        let target = crate::project::files::inspect(path, &self.cancelled)?;
        Ok(self
            .project
            .as_ref()
            .filter(|project| project.target.sha256 == target.sha256)
            .map(|_| target.sha256))
    }

    pub(super) fn execution_evidence(&mut self, args: &Value) -> Result<Value> {
        let project = self.project.as_ref().ok_or("Open a native file first")?;
        if !matches!(project.target.kind.as_str(), "app" | "binary") {
            return Err(failure(
                "Execution evidence currently requires a native binary",
            ));
        }
        let project = project.target.sha256.clone();
        match args["action"].as_str() {
            Some("import") => self.import_execution_trace(&project, args),
            Some("capture_debugger") => self.capture_debugger_evidence(&project, args),
            Some("list") => {
                let traces = self.store.execution_traces(&project)?;
                let summaries: Vec<_> = traces.iter().map(|trace| json!({
                    "id":trace.id,"workflow":trace.workflow,"source":trace.source,
                    "runtime_version":trace.runtime_version,"captured_at":trace.captured_at,
                    "complete":trace.complete,"observations":trace.events.len(),"mapping":trace.mapping
                })).collect();
                Ok(json!({"traces":summaries}))
            }
            Some("read") => Ok(serde_json::to_value(self.execution_trace(&project, args)?)?),
            Some("functions") => self.executed_function_page(&project, args),
            Some("remove") => {
                let id = required_text(args, "trace_id")?;
                self.store.remove_execution_trace(&project, id)?;
                self.sink
                    .send(crate::events::Event::ExecutionEvidenceChanged(project))?;
                Ok(json!({"removed":id}))
            }
            Some("graph") => {
                let address = required_text(args, "address")?;
                self.require_indexed_function(address)?;
                let graph = self.read_view(View::Graph, address, 0)?;
                Ok(serde_json::from_str(&graph)?)
            }
            _ => Err(failure("Unknown execution-evidence action")),
        }
    }

    fn import_execution_trace(&self, project: &str, args: &Value) -> Result<Value> {
        let value = args.get("trace").ok_or("A trace object is required")?;
        if serde_json::to_vec(value)?.len() > MAX_TRACE_BYTES {
            return Err(failure("Trace exceeds the 2 MiB limit"));
        }
        let trace: ExecutionTrace = serde_json::from_value(value.clone())?;
        self.store.save_execution_trace(project, &trace)?;
        self.sink
            .send(crate::events::Event::ExecutionEvidenceChanged(
                project.to_owned(),
            ))?;
        Ok(json!({"trace_id":trace.id,"observations":trace.events.len()}))
    }

    fn execution_trace(&self, project: &str, args: &Value) -> Result<ExecutionTrace> {
        let id = required_text(args, "trace_id")?;
        self.store
            .execution_traces(project)?
            .into_iter()
            .find(|trace| trace.id == id)
            .ok_or_else(|| failure(format!("Unknown trace: {id}")))
    }

    fn executed_function_page(&self, project: &str, args: &Value) -> Result<Value> {
        let trace = self.execution_trace(project, args)?;
        let offset = usize::try_from(args["offset"].as_u64().unwrap_or_default())?;
        let (functions, total) = self.store.executed_functions(project, &trace, offset)?;
        let next = offset.saturating_add(functions.len());
        Ok(
            json!({"trace_id":trace.id,"functions":functions,"total":total,
            "next_offset":(next < total).then_some(next)}),
        )
    }

    fn capture_debugger_evidence(&mut self, project: &str, args: &Value) -> Result<Value> {
        // This command only samples a stopped session created explicitly in the UI.
        // It never launches, attaches, resumes, evaluates expressions, or steps.
        let mut trace = ExecutionTrace {
            version: 1,
            id: required_text(args, "trace_id")?.to_owned(),
            workflow: required_text(args, "workflow")?.to_owned(),
            source: TraceSource::Debugger,
            runtime_version: required_text(args, "runtime_version")?.to_owned(),
            captured_at: format!("unix-ms:{}", crate::model::now_ms()),
            target_sha256: project.to_owned(),
            mapping: serde_json::from_value::<ModuleMapping>(
                args.get("mapping")
                    .cloned()
                    .ok_or("A module mapping is required")?,
            )?,
            complete: false,
            events: Vec::new(),
        };
        let thread = args["thread_id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or("A positive stopped thread ID is required")?;
        let debugger = self
            .debugger
            .as_mut()
            .ok_or("Connect a debugger in the UI first")?;
        trace.events.push(debugger.capture_sample(project, thread)?);
        self.store.save_execution_trace(project, &trace)?;

        // Keep the previously verified evidence until a replacement sample has
        // been captured and persisted. A missing debugger or a failed sample
        // must leave the existing evidence usable.

        self.sink
            .send(crate::events::Event::ExecutionEvidenceChanged(
                project.to_owned(),
            ))?;
        Ok(json!({"trace_id":trace.id,"observations":trace.events.len(),"complete":false}))
    }
}

fn required_text<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| failure(format!("A nonempty {key} is required")))
}
