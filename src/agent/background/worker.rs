use super::{Prompt, Request};
use crate::agent::session::{self, Control, Session};
use crate::error::Result;
use crate::events::{Event, Sink};
use crate::model::AgentUpdate;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

pub(super) fn run(
    requests: mpsc::Receiver<Request>,
    controls: mpsc::Receiver<Control>,
    sink: Sink,
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    preparing: Arc<AtomicBool>,
) {
    let mut worker = Worker {
        requests,
        controls,
        sink,
        cancelled,
        shutdown,
        preparing,
        session: None,
        last_prompt: Instant::now(),
    };
    // Losing the UI receiver is the only reason to stop delivering events.
    if let Err(error) = worker.run() {
        eprintln!("The agent worker closed: {error}");
    }
}

struct Worker {
    requests: mpsc::Receiver<Request>,
    controls: mpsc::Receiver<Control>,
    sink: Sink,
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    preparing: Arc<AtomicBool>,
    session: Option<Session>,
    last_prompt: Instant,
}

impl Worker {
    fn run(&mut self) -> Result<()> {
        while !self.shutdown.load(Ordering::Relaxed) {
            match self.next_request() {
                Ok(Request::Prompt(prompt)) => self.run_prompt(prompt)?,
                Ok(Request::Prepare(prompt)) => self.prepare_session(prompt)?,
                Err(mpsc::RecvTimeoutError::Timeout) => self.maintain_session()?,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        self.close_session()
    }

    fn next_request(&self) -> std::result::Result<Request, mpsc::RecvTimeoutError> {
        if self.session.is_none() {
            return self.requests.recv_timeout(Duration::from_millis(100));
        }
        self.requests.try_recv().map_err(|error| match error {
            mpsc::TryRecvError::Empty => mpsc::RecvTimeoutError::Timeout,
            mpsc::TryRecvError::Disconnected => mpsc::RecvTimeoutError::Disconnected,
        })
    }

    fn prepare_session(&mut self, prompt: Prompt) -> Result<()> {
        let result = session::prepare(
            &mut self.session,
            &prompt,
            &self.sink,
            &self.cancelled,
            self.shutdown.clone(),
            &self.controls,
        );
        self.preparing.store(false, Ordering::Relaxed);
        self.last_prompt = Instant::now();
        if let Err(error) = result {
            self.close_session()?;
            self.sink
                .send(Event::AgentUpdate(AgentUpdate::Notice(format!(
                    "Couldn't load agent commands: {error}"
                ))))?;
        }
        Ok(())
    }

    fn run_prompt(&mut self, prompt: Prompt) -> Result<()> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }
        let keep_running = prompt.settings.conversation.keep_agents_running;
        let result = session::run(
            &mut self.session,
            prompt,
            &self.sink,
            self.cancelled.clone(),
            self.shutdown.clone(),
            &self.controls,
        );
        self.last_prompt = Instant::now();
        if result.is_err() || !keep_running {
            self.close_session()?;
        }
        self.sink.send(Event::AgentFinished {
            result,
            stopped: self.cancelled.load(Ordering::Relaxed),
        })
    }

    fn maintain_session(&mut self) -> Result<()> {
        let Some(active) = &mut self.session else {
            while let Ok(control) = self.controls.try_recv() {
                session::unavailable(control, &self.sink)?;
            }
            return Ok(());
        };
        if let Err(error) = active.idle(&self.controls, &self.sink, &self.cancelled) {
            self.close_session()?;
            self.sink
                .send(Event::AgentUpdate(AgentUpdate::Notice(format!(
                    "The agent session closed: {error}"
                ))))?;
        }
        if self.last_prompt.elapsed() > Duration::from_secs(600)
            && self
                .session
                .as_ref()
                .is_some_and(|session| !session.has_active_goal())
        {
            self.close_session()?;
        }
        Ok(())
    }

    fn close_session(&mut self) -> Result<()> {
        let Some(mut session) = self.session.take() else {
            return Ok(());
        };
        crate::error::finish(
            session.close(),
            self.sink
                .send(Event::AgentUpdate(AgentUpdate::SessionClosed)),
            "Reporting the closed agent session",
        )
    }
}
