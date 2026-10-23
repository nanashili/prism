use super::{Backend, Engine};
use crate::analysis::{DecompilationBatch, DecompilationBatchPoll, DecompilationItem};
use crate::error::{Result, failure};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

impl Engine {
    pub fn collect_decompilation_batch(
        &mut self,
        addresses: Vec<String>,
        cancelled: &AtomicBool,
    ) -> Result<Vec<DecompilationItem>> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        let mut batch = self.begin_decompilation_batch(addresses)?;
        let result = self.collect_batch_pages(&mut batch, cancelled);
        let release = self.abandon_decompilation_batch(batch);

        match (result, release) {
            (Ok(items), Ok(())) => Ok(items),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(release)) => Err(failure(format!(
                "{error}; releasing the decompilation batch also failed: {release}"
            ))),
        }
    }

    fn collect_batch_pages(
        &mut self,
        batch: &mut DecompilationBatch,
        cancelled: &AtomicBool,
    ) -> Result<Vec<DecompilationItem>> {
        let mut items = Vec::new();
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }

            match self.poll_decompilation_batch(batch)? {
                DecompilationBatchPoll::Pending => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                DecompilationBatchPoll::Page(page) => items.extend(page),
                DecompilationBatchPoll::Complete => return Ok(items),
            }
        }
    }

    pub fn begin_decompilation_batch(
        &mut self,
        addresses: Vec<String>,
    ) -> Result<DecompilationBatch> {
        self.last_used = Instant::now();
        match &mut self.backend {
            Backend::Rea(engine) => engine.begin_decompilation_batch(addresses),
            _ => Err(failure(
                "Retained provider batches require a Ghidra REA provider",
            )),
        }
    }

    pub fn poll_decompilation_batch(
        &mut self,
        batch: &mut DecompilationBatch,
    ) -> Result<DecompilationBatchPoll> {
        self.last_used = Instant::now();
        match &mut self.backend {
            Backend::Rea(engine) => engine.poll_decompilation_batch(batch),
            _ => Err(failure("The prepared Ghidra REA session is unavailable")),
        }
    }

    pub fn abandon_decompilation_batch(&mut self, batch: DecompilationBatch) -> Result<()> {
        self.last_used = Instant::now();
        match &mut self.backend {
            Backend::Rea(engine) => engine.abandon_decompilation_batch(batch),
            _ => Err(failure(
                "The decompilation batch belongs to a different engine",
            )),
        }
    }
}
