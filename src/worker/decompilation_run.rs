use super::State;
use crate::decompilation::ExportRunner;
use crate::error::Result;
use std::sync::atomic::Ordering;

#[cfg(feature = "rea-rust-ghidra")]
mod retained;

impl State {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn run_decompilation_step(&mut self) -> bool {
        let Some(mut runner) = self.decompilation.take() else {
            return false;
        };
        match self.advance_decompilation(&mut runner) {
            Ok(progress) => {
                if runner.is_running() {
                    self.decompilation = Some(runner);
                } else if let Err(error) = self.release_decompilation_runner(runner) {
                    eprintln!("Could not release the inactive durable job: {error}");
                }
                progress
            }
            Err(error) => {
                self.pause_failed_decompilation(runner, &error.to_string());
                true
            }
        }
    }

    fn pause_failed_decompilation(&mut self, mut runner: ExportRunner, message: &str) {
        eprintln!("Durable decompilation paused: {message}");
        if let Err(error) = runner.pause(&self.store, Some(message)) {
            eprintln!("Could not save the pause after {message}: {error}");
        }
        if let Err(error) = self.release_decompilation_runner(runner) {
            eprintln!("Could not release the paused durable job after {message}: {error}");
        }

        if let Err(error) = self.discard_unusable_engine() {
            eprintln!("Could not close the failed decompilation session after {message}: {error}");
        }
    }

    fn advance_decompilation(&mut self, runner: &mut ExportRunner) -> Result<bool> {
        if self.cancelled.load(Ordering::Relaxed) {
            runner.pause(&self.store, None)?;
            return Ok(true);
        }
        let context = self.decompilation_context()?;
        if !runner.matches_context(&context) {
            runner.pause(
                &self.store,
                Some("The project or analysis configuration changed"),
            )?;
            return Ok(true);
        }
        #[cfg(feature = "rea-rust-ghidra")]
        if self.settings.analysis_backend().retains_prepared_session() {
            return self.advance_retained_decompilation(runner);
        }

        self.advance_single_decompilation(runner)?;
        Ok(true)
    }

    fn advance_single_decompilation(&mut self, runner: &mut ExportRunner) -> Result<()> {
        let Some(address) = runner.next_address(&self.store)? else {
            return runner.complete(&self.store);
        };
        if runner.skip_saved(&self.store, &address)? {
            return Ok(());
        }

        let result = self.read_durable_pseudocode(&address);
        if self.engine.is_none() && result.is_err() {
            // Opening/restoring failed before a function could run. Pause at
            // this address instead of retrying the same restore for every item.
            return result.map(|_| ());
        }
        let result = crate::error::finish(
            result,
            self.discard_unusable_engine(),
            "Closing the decompilation session",
        );
        if self.cancelled.load(Ordering::Relaxed) {
            return runner.pause(&self.store, None);
        }
        runner.record_result(&self.store, &address, result)
    }
}
