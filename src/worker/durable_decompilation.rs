use super::State;
use crate::decompilation::{Context, ExportRunner};
use crate::error::{Result, failure};
use serde_json::Value;

impl State {
    pub(super) fn decompilation_context(&self) -> Result<Context> {
        let project = &self
            .project
            .as_ref()
            .ok_or("Open a project first")?
            .target
            .sha256;
        let context = self.runtime.current_context()?;
        if context.project != *project || context.provider != self.settings.provider {
            return Err(failure(
                "The prepared analysis configuration is unavailable. Reopen the project.",
            ));
        }
        Ok(context)
    }

    pub(super) fn durable_decompile_batch(&mut self, args: &Value) -> Result<Value> {
        let action = args["action"]
            .as_str()
            .ok_or("Supply a durable job action")?;
        let context = self.decompilation_context()?;
        if action == "list" {
            return ExportRunner::list(&self.store, &context, args);
        }
        if action == "start" {
            self.require_available_runner(None)?;
            let mut result = ExportRunner::start(&self.store, &context, args)?;
            let id = result["job"].as_str().ok_or("The durable job has no id")?;
            if result["created"] == true {
                self.activate_decompilation(id, &context)?;
            }
            result["status"] = ExportRunner::status(&self.store, id, &context)?;
            return Ok(result);
        }

        let id = ExportRunner::parse_job_id(args)?;
        match action {
            "resume" => {
                self.require_available_runner(Some(id))?;
                if self.decompilation.is_none() {
                    self.activate_decompilation(id, &context)?;
                }
                ExportRunner::status(&self.store, id, &context)
            }
            "status" => {
                let mut status = ExportRunner::status(&self.store, id, &context)?;
                status["active_in_window"] = serde_json::json!(
                    self.decompilation
                        .as_ref()
                        .is_some_and(|runner| runner.id() == id)
                );
                Ok(status)
            }
            "results" => ExportRunner::results(&self.store, id, &context, args),
            "pause" | "cancel" | "retry_failed" => {
                if action == "retry_failed" {
                    self.require_available_runner(None)?;
                }
                let response = if let Some(runner) = self
                    .decompilation
                    .as_mut()
                    .filter(|runner| runner.id() == id)
                {
                    let response = runner.control(&self.store, &context, action)?;
                    let runner = self
                        .decompilation
                        .take()
                        .ok_or("The active job is missing")?;
                    self.release_decompilation_runner(runner)?;
                    response
                } else {
                    ExportRunner::control_saved(&self.store, id, &context, action)?
                };
                Ok(response)
            }
            _ => Err(failure("Unknown decompile_batch action")),
        }
    }

    fn activate_decompilation(&mut self, id: &str, context: &Context) -> Result<()> {
        let mut runner = ExportRunner::resume(&self.store, id, context)?;
        if runner.next_address(&self.store)?.is_none() {
            runner.complete(&self.store)?;
            self.runtime.set_active_job(None)?;
        } else {
            self.decompilation = Some(runner);
            self.runtime.set_active_job(Some(id))?;
        }
        Ok(())
    }

    fn require_available_runner(&self, requested: Option<&str>) -> Result<()> {
        if let Some(runner) = &self.decompilation
            && requested != Some(runner.id())
        {
            return Err(failure(format!(
                "Durable job {} is active. Pause it before running another job.",
                runner.id()
            )));
        }
        Ok(())
    }

    pub(super) fn pause_decompilation(&mut self) -> Result<()> {
        if let Some(runner) = self.decompilation.as_mut() {
            runner.pause(&self.store, None)?;
        }
        match self.decompilation.take() {
            Some(runner) => self.release_decompilation_runner(runner),
            None => self.runtime.set_active_job(None),
        }
    }

    pub(super) fn release_decompilation_runner(&mut self, runner: ExportRunner) -> Result<()> {
        #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
        let release = {
            let mut runner = runner;
            self.release_decompilation_batch(&mut runner)
        };
        #[cfg(not(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra")))]
        let release: Result<()> = {
            drop(runner);
            Ok(())
        };

        let clear = self.runtime.set_active_job(None);
        match (release, clear) {
            (Ok(()), result) | (result, Ok(())) => result,
            (Err(release), Err(clear)) => Err(failure(format!(
                "{release}; could not publish the inactive durable job: {clear}"
            ))),
        }
    }
}
