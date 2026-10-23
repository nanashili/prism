use crate::decompilation::Context;
use crate::error::{Result, failure};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub(crate) struct RuntimeSnapshot {
    state: Arc<RwLock<RuntimeState>>,
}

struct RuntimeState {
    context: Option<Context>,
    active_job: Option<String>,
    transitioning: bool,
}

pub(crate) struct Transition {
    snapshot: RuntimeSnapshot,
}

impl RuntimeSnapshot {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(RuntimeState {
                context: None,
                active_job: None,
                transitioning: false,
            })),
        }
    }

    pub(crate) fn begin_transition(&self) -> Result<Transition> {
        let mut state = self.write_state()?;
        state.transitioning = true;
        Ok(Transition {
            snapshot: self.clone(),
        })
    }

    pub(crate) fn set_context(&self, context: Context) -> Result<()> {
        let mut state = self.write_state()?;
        state.context = Some(context);
        Ok(())
    }

    pub(crate) fn set_active_job(&self, id: Option<&str>) -> Result<()> {
        let mut state = self.write_state()?;
        state.active_job = id.map(str::to_owned);
        Ok(())
    }

    pub(crate) fn read(&self) -> Result<(Context, Option<String>)> {
        let state = self
            .state
            .read()
            .map_err(|_| failure("The project runtime state is unavailable"))?;
        if state.transitioning {
            return Err(failure("The project is changing; retry the durable query"));
        }
        let context = state
            .context
            .clone()
            .ok_or("Open a project before querying durable decompilation")?;
        Ok((context, state.active_job.clone()))
    }

    /// The owning worker may use its prepared context during an explicit open.
    /// External readers continue to reject queries until that transition ends.
    pub(super) fn current_context(&self) -> Result<Context> {
        self.state
            .read()
            .map_err(|_| failure("The project runtime state is unavailable"))?
            .context
            .clone()
            .ok_or_else(|| failure("Open a project before using analysis"))
    }

    fn write_state(&self) -> Result<std::sync::RwLockWriteGuard<'_, RuntimeState>> {
        self.state
            .write()
            .map_err(|_| failure("The project runtime state is unavailable"))
    }
}

impl Drop for Transition {
    fn drop(&mut self) {
        match self.snapshot.state.write() {
            Ok(mut state) => state.transitioning = false,
            Err(_) => eprintln!("Could not clear the project transition state"),
        }
    }
}
