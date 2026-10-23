use super::State;
use crate::analysis::{DecompilationBatchPoll, Engine};
use crate::decompilation::{BatchWindow, ExportRunner};
use crate::error::{Result, failure};
use crate::settings::AnalysisBackend;
use std::collections::VecDeque;

impl State {
    pub(super) fn advance_retained_decompilation(
        &mut self,
        runner: &mut ExportRunner,
    ) -> Result<bool> {
        let Some(batch) = runner.batch.as_ref() else {
            return self.begin_retained_decompilation(runner);
        };
        if !batch.items.is_empty() {
            return self.persist_retained_decompilation(runner);
        }

        let batch = runner
            .batch
            .as_mut()
            .ok_or("The provider batch is missing")?;
        match self
            .prepared_retained_engine()?
            .poll_decompilation_batch(&mut batch.handle)?
        {
            DecompilationBatchPoll::Pending => Ok(false),
            DecompilationBatchPoll::Page(items) => {
                batch.retain_page(items)?;
                Ok(true)
            }
            DecompilationBatchPoll::Complete => {
                if !batch.positions.is_empty() {
                    return Err(failure("The provider batch omitted frozen job addresses"));
                }
                self.release_decompilation_batch(runner)?;
                Ok(true)
            }
        }
    }

    fn begin_retained_decompilation(&mut self, runner: &mut ExportRunner) -> Result<bool> {
        let Some(address) = runner.next_address(&self.store)? else {
            runner.complete(&self.store)?;
            return Ok(true);
        };
        if runner.skip_saved(&self.store, &address)? {
            return Ok(true);
        }
        let limit = match self.settings.analysis_backend() {
            AnalysisBackend::RustGhidraRea => 64,
            _ => 256,
        };
        let items = runner.missing_batch_items(&self.store, limit)?;
        if items.first().is_none_or(|(position, item)| {
            *position != runner.current_position() || item != &address
        }) {
            return Err(failure(
                "The provider batch does not begin at the durable cursor",
            ));
        }
        let (positions, addresses): (VecDeque<_>, Vec<_>) = items.into_iter().unzip();
        let handle = self
            .prepared_retained_engine()?
            .begin_decompilation_batch(addresses)?;
        runner.batch = Some(BatchWindow::new(handle, positions));
        Ok(true)
    }

    fn persist_retained_decompilation(&mut self, runner: &mut ExportRunner) -> Result<bool> {
        let address = runner
            .next_address(&self.store)?
            .ok_or("The provider returned results beyond the durable manifest")?;
        if runner.skip_saved(&self.store, &address)? {
            return Ok(true);
        }
        let position = runner.current_position();
        let batch = runner
            .batch
            .as_mut()
            .ok_or("The provider batch is missing")?;
        let expected = batch
            .positions
            .front()
            .copied()
            .ok_or("The provider batch has no selector")?;
        if position < expected {
            // A row marked successful was excluded from the request, but its
            // integrity check now failed. Recompute it in a fresh bounded batch.
            self.release_decompilation_batch(runner)?;
            return Ok(true);
        }
        if position != expected {
            return Err(failure(
                "The provider result does not match the frozen job cursor",
            ));
        }
        self.record_retained_batch_item(runner, &address)?;
        Ok(true)
    }

    fn record_retained_batch_item(&self, runner: &mut ExportRunner, address: &str) -> Result<()> {
        let batch = runner
            .batch
            .as_mut()
            .ok_or("The provider batch is missing")?;
        if batch
            .items
            .front()
            .is_none_or(|item| item.address != address)
        {
            return Err(failure(
                "The provider result address does not match the frozen job cursor",
            ));
        }
        let item = batch
            .items
            .pop_front()
            .ok_or("The provider result page is empty")?;
        runner.record_result_with_details(
            &self.store,
            address,
            item.result.map_err(failure),
            item.error_details,
        )?;
        runner
            .batch
            .as_mut()
            .ok_or("The provider batch is missing")?
            .positions
            .pop_front()
            .ok_or("The provider batch selector is missing")?;
        Ok(())
    }

    fn prepared_retained_engine(&mut self) -> Result<&mut Engine> {
        self.engine.as_mut().filter(|engine| engine.is_reusable()).ok_or_else(|| {
            failure("The Ghidra provider needs an already analyzed session. Open the project and explicitly run Analyze before resuming this job.")
        })
    }

    pub(in crate::worker) fn release_decompilation_batch(
        &mut self,
        runner: &mut ExportRunner,
    ) -> Result<()> {
        let Some(batch) = runner.batch.take() else {
            return Ok(());
        };
        let Some(engine) = self.engine.as_mut() else {
            // Dropping the whole engine already closed its provider connection.
            return Ok(());
        };
        engine.abandon_decompilation_batch(batch.handle)
    }
}
