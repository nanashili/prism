use super::State;
use crate::error::Result;
use crate::events::Event;

impl State {
    pub(super) fn index(&mut self) -> Result<String> {
        self.sink.analysis("Starting analysis", 0, None)?;
        let project = self.project.as_ref().ok_or("Open a file first")?;
        let android = project.target.kind == "android";
        let count = if android {
            crate::analysis::android::index(
                &project.target,
                &self.settings,
                &mut self.store,
                &self.cancelled,
                &self.sink,
            )?
        } else {
            self.prepare_engine()?;
            let project = self.project.as_ref().ok_or("Open a file first")?;
            self.engine
                .as_mut()
                .ok_or("The analysis engine is unavailable")?
                .index(&mut self.store, &project.target.sha256)?
        };

        let project = self.project.as_ref().ok_or("Open a file first")?;
        let rows = self.store.functions(&project.target.sha256, "", 0)?;
        self.sink.send(Event::Functions { rows, offset: 0 })?;
        let unit = if android {
            "Android methods"
        } else {
            "functions"
        };
        Ok(format!("{count} {unit} indexed"))
    }
}
