use super::{Session, control};
use crate::agent::Prompt;
use crate::error::{Result, failure};
use crate::events::Sink;
use serde_json::json;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Receiver,
};
use std::time::{Duration, Instant};

impl Session {
    pub(super) fn prompt(
        &mut self,
        prompt: &Prompt,
        history: &str,
        sink: &Sink,
        cancelled: &AtomicBool,
        controls: &Receiver<control::Control>,
    ) -> Result<()> {
        let result = self.run_turn(prompt, history, sink, cancelled, controls);
        self.finish_permissions(result, sink)
    }

    fn run_turn(
        &mut self,
        prompt: &Prompt,
        history: &str,
        sink: &Sink,
        cancelled: &AtomicBool,
        controls: &Receiver<control::Control>,
    ) -> Result<()> {
        let content = crate::agent::prompt_content::build(prompt, history, self.images)?;
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        let id = self.rpc.begin(
            "session/prompt",
            json!({"sessionId":self.id,"prompt":content}),
        )?;
        let started = Instant::now();
        let mut cancellation_sent = None;

        loop {
            if self.rpc.is_cancelled() {
                return Err(failure("Cancelled"));
            }
            if started.elapsed() > Duration::from_secs(3600) && !self.updates.goal_active {
                return Err(failure("session/prompt timed out"));
            }
            self.request_cancellation(cancelled, &mut cancellation_sent)?;
            self.poll_permissions(sink, cancelled)?;
            self.send_controls(controls, sink)?;

            let Some(mut message) = self.rpc.poll(Duration::from_millis(100))? else {
                continue;
            };
            if crate::protocol::response_id(&message) == Some(id) {
                let response = crate::protocol::response_result(message)?;
                if response["stopReason"] == "cancelled" {
                    cancelled.store(true, Ordering::Relaxed);
                }
                return Ok(());
            }
            if !self.control_reply(&mut message, sink)? {
                self.receive(message, sink, cancelled)?;
            }
        }
    }

    fn request_cancellation(
        &mut self,
        cancelled: &AtomicBool,
        sent: &mut Option<Instant>,
    ) -> Result<()> {
        if cancelled.load(Ordering::Relaxed) && sent.is_none() {
            self.rpc
                .notify("session/cancel", json!({"sessionId":self.id}))?;
            *sent = Some(Instant::now());
        }
        if sent.is_some_and(|at| at.elapsed() > Duration::from_secs(15)) {
            return Err(failure(
                "The agent did not acknowledge Stop within 15 seconds",
            ));
        }
        Ok(())
    }

    pub(in crate::agent) fn idle(
        &mut self,
        controls: &Receiver<control::Control>,
        sink: &Sink,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.poll_permissions(sink, cancelled)?;
        self.send_controls(controls, sink)?;
        if let Some(mut message) = self.rpc.poll(Duration::from_millis(100))?
            && !self.control_reply(&mut message, sink)?
        {
            self.receive(message, sink, cancelled)?;
        }
        Ok(())
    }
}
