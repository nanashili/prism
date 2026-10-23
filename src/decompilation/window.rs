use super::ExportRunner;
use crate::analysis::{DecompilationBatch, DecompilationItem};
use crate::error::{Result, failure};
use crate::storage::Store;
use std::collections::VecDeque;

const PAGE_LIMIT: usize = 64;

pub(crate) struct BatchWindow {
    pub(crate) handle: DecompilationBatch,
    pub(crate) items: VecDeque<DecompilationItem>,
    pub(crate) positions: VecDeque<usize>,
}

impl BatchWindow {
    pub(crate) fn new(handle: DecompilationBatch, positions: VecDeque<usize>) -> Self {
        Self {
            handle,
            items: VecDeque::new(),
            positions,
        }
    }

    pub(crate) fn retain_page(&mut self, items: Vec<DecompilationItem>) -> Result<()> {
        if !self.items.is_empty() || items.is_empty() || items.len() > PAGE_LIMIT {
            return Err(failure(
                "The provider returned an invalid durable result page",
            ));
        }
        self.items = items.into();
        Ok(())
    }
}

impl ExportRunner {
    pub(crate) fn missing_batch_items(
        &self,
        store: &Store,
        limit: usize,
    ) -> Result<Vec<(usize, String)>> {
        store.decompilation_missing_window(&self.job.id, self.job.next_offset, limit)
    }

    pub(crate) fn current_position(&self) -> usize {
        self.job.next_offset
    }
}
