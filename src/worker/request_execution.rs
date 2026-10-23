use super::{Job, RequestLifetime, State};
use crate::error::{Result, failure};
use crate::events::Event;
use serde_json::Value;

impl State {
    pub(super) fn finish_job(&mut self, job: Job) -> bool {
        if let Job::Remote {
            tool,
            args,
            answer,
            lifetime,
        } = job
        {
            let result = self.execute_remote(&tool, args, lifetime.as_ref());
            self.discard_unusable_engine();
            if answer.send(result).is_err() {
                eprintln!("The agent no longer needs the analysis result");
            }
            return self.sink.send(Event::Analysis(None)).is_ok();
        }
        let result = self.execute(job);
        self.discard_unusable_engine();
        if self.sink.send(Event::Analysis(None)).is_err() {
            return false;
        }
        self.sink.send(Event::Finished(result)).is_ok()
    }

    fn execute_remote(
        &mut self,
        tool: &str,
        args: Value,
        lifetime: Option<&RequestLifetime>,
    ) -> Result<Value> {
        let active = lifetime
            .map(|request| request.activate(self.cancelled.clone()))
            .transpose()?;
        let result = self.remote(tool, args);
        let cleanup = active.map(|active| active.finish()).transpose();
        match (result, cleanup) {
            (Err(primary), Err(cleanup)) => {
                Err(failure(format!("{primary}\nTool cleanup: {cleanup}")))
            }
            (Err(error), Ok(_)) | (Ok(_), Err(error)) => Err(error),
            (Ok(value), Ok(_)) => Ok(value),
        }
    }
}
