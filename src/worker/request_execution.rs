use super::{Job, RequestLifetime, State};
use crate::error::Result;
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
            let result = crate::error::finish(
                result,
                self.discard_unusable_engine(),
                "Closing the analysis session",
            );
            if answer.send(result).is_err() {
                eprintln!("The agent no longer needs the analysis result");
            }
            return self.sink.send(Event::Analysis(None)).is_ok();
        }
        let result = self.execute(job);
        let result = crate::error::finish(
            result,
            self.discard_unusable_engine(),
            "Closing the analysis session",
        );
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
        let cleanup = active
            .map(super::request_lifetime::ActiveRequest::finish)
            .transpose();
        crate::error::finish(result, cleanup.map(|_| ()), "Tool cleanup")
    }
}
