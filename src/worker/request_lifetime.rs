use crate::error::{Result, failure};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

#[derive(Clone)]
pub(crate) struct RequestLifetime {
    state: Arc<RequestState>,
}

struct RequestState {
    cancelled: AtomicBool,
    deadline: Instant,
    execution: Mutex<Option<Arc<AtomicBool>>>,
}

pub(super) struct ActiveRequest {
    lifetime: Option<RequestLifetime>,
}

impl RequestLifetime {
    pub(crate) fn new(deadline: Instant) -> Self {
        Self {
            state: Arc::new(RequestState {
                cancelled: AtomicBool::new(false),
                deadline,
                execution: Mutex::new(None),
            }),
        }
    }

    pub(crate) fn check(&self) -> Result<()> {
        if Instant::now() >= self.state.deadline {
            return Err(failure("The tool request timed out"));
        }
        if self.state.cancelled.load(Ordering::Acquire) {
            return Err(failure("The tool request was cancelled"));
        }
        Ok(())
    }

    pub(crate) fn cancel(&self) -> Result<()> {
        self.state.cancelled.store(true, Ordering::Release);
        let execution = self
            .state
            .execution
            .lock()
            .map_err(|_| failure("The tool cancellation state is unavailable"))?;
        if let Some(execution) = execution.as_ref() {
            execution.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    pub(super) fn activate(&self, execution: Arc<AtomicBool>) -> Result<ActiveRequest> {
        let mut active = self
            .state
            .execution
            .lock()
            .map_err(|_| failure("The tool cancellation state is unavailable"))?;
        self.check()?;
        if active.is_some() {
            return Err(failure("The tool request is already executing"));
        }
        *active = Some(execution);
        Ok(ActiveRequest {
            lifetime: Some(self.clone()),
        })
    }

    fn deactivate(&self) -> Result<()> {
        let mut active = self
            .state
            .execution
            .lock()
            .map_err(|_| failure("The tool cancellation state is unavailable"))?;
        *active = None;
        Ok(())
    }
}

impl ActiveRequest {
    pub(super) fn finish(mut self) -> Result<()> {
        if let Some(lifetime) = self.lifetime.take() {
            lifetime.deactivate()?;
        }
        Ok(())
    }
}

impl Drop for ActiveRequest {
    fn drop(&mut self) {
        if let Some(lifetime) = self.lifetime.take()
            && let Err(error) = lifetime.deactivate()
        {
            eprintln!("Clearing tool cancellation state failed: {error}");
        }
    }
}
