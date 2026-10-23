use super::{Job, State};
use crate::error::Result;
use crate::events::Event;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

const ORDINARY_SHARE: usize = 2;
const CONTROL_SHARE: usize = 2;
const CONTROL_POLL: Duration = Duration::from_millis(25);

impl State {
    pub(super) fn run(
        &mut self,
        ordinary: mpsc::Receiver<Job>,
        controls: mpsc::Receiver<Job>,
        stopped: &AtomicBool,
    ) {
        while !stopped.load(Ordering::Relaxed) {
            if !self.run_round(&ordinary, &controls, stopped) {
                break;
            }
        }
        if let Err(error) = self.pause_decompilation() {
            eprintln!("Could not checkpoint the durable job on shutdown: {error}");
        }
    }

    fn run_round(
        &mut self,
        ordinary: &mpsc::Receiver<Job>,
        controls: &mpsc::Receiver<Job>,
        stopped: &AtomicBool,
    ) -> bool {
        let mut worked = false;
        for (queue, share) in [(controls, CONTROL_SHARE), (ordinary, ORDINARY_SHARE)] {
            for _ in 0..share {
                if stopped.load(Ordering::Relaxed) {
                    return false;
                }
                match queue.try_recv() {
                    Ok(job) => {
                        worked = true;
                        if !self.run_job(job) {
                            return false;
                        }
                    }
                    Err(mpsc::TryRecvError::Disconnected) => return false,
                    Err(mpsc::TryRecvError::Empty) => break,
                }
            }
        }
        if stopped.load(Ordering::Relaxed) {
            return false;
        }
        if self.decompilation.is_some() {
            if let Err(error) = self.prepare_execution() {
                if let Err(send_error) = self.sink.send(Event::Finished(Err(error))) {
                    eprintln!("Reporting the checkpoint failure failed: {send_error}");
                }
                return false;
            }
            let progress = self.run_decompilation_step();
            if !progress && !worked {
                return self.wait_for_job(ordinary, stopped);
            }
            return true;
        }
        if worked {
            return true;
        }
        self.wait_for_job(ordinary, stopped)
    }

    fn wait_for_job(&mut self, ordinary: &mpsc::Receiver<Job>, stopped: &AtomicBool) -> bool {
        if self
            .engine
            .as_ref()
            .is_some_and(|engine| engine.idle_remaining().is_zero())
        {
            self.engine = None;
        }
        match ordinary.recv_timeout(CONTROL_POLL) {
            Ok(job) if !stopped.load(Ordering::Relaxed) => self.run_job(job),
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => false,
            Err(mpsc::RecvTimeoutError::Timeout) => true,
        }
    }

    fn run_job(&mut self, job: Job) -> bool {
        if let Err(error) = self.prepare_execution() {
            if let Job::Remote { answer, .. } = job {
                if answer.send(Err(error)).is_err() {
                    eprintln!("The tool caller closed before receiving the checkpoint failure");
                }
            } else if let Err(send_error) = self.sink.send(Event::Finished(Err(error))) {
                eprintln!("Reporting the checkpoint failure failed: {send_error}");
            }
            return false;
        }
        self.finish_job(job)
    }

    fn prepare_execution(&mut self) -> Result<()> {
        self.cancelled.store(false, Ordering::Relaxed);
        let requested = self.stop_generation.load(Ordering::Acquire);
        if requested != self.observed_stop {
            self.pause_decompilation()?;
            self.observed_stop = requested;
        }
        Ok(())
    }
}
