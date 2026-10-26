use super::State;
use crate::error::{Result, failure};
use crate::events::Event;
use crate::investigation::Budget;

impl State {
    pub(super) fn start_investigation(
        &self,
        project: &str,
        objective: &str,
        budget: Budget,
    ) -> Result<String> {
        let opened = self.project.as_ref().ok_or("Open a file first")?;
        if opened.target.sha256 != project {
            return Err(failure(
                "The open project changed before investigation planning",
            ));
        }
        let graph = crate::investigation::start(&self.store, project, objective, budget)?;
        self.sink
            .send(Event::InvestigationCreated(Box::new(graph)))?;
        Ok("Investigation saved".into())
    }
}
