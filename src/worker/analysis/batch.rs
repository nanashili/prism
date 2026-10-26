use super::State;
use crate::analysis::DecompilationItem;
use crate::error::{Result, failure};

impl State {
    pub(in crate::worker) fn saved_decompilation_batch(
        &mut self,
        addresses: Vec<String>,
    ) -> Result<Vec<DecompilationItem>> {
        for address in &addresses {
            self.require_indexed_function(address)?;
        }

        let cancelled = self.cancelled.clone();
        let result = self
            .engine()?
            .collect_decompilation_batch(addresses, &cancelled);
        crate::error::finish(
            result,
            self.discard_unusable_engine(),
            "Closing the analysis session",
        )
    }

    pub(in crate::worker) fn batch_pseudocode(&mut self, address: &str) -> Result<String> {
        let items = self.saved_decompilation_batch(vec![address.to_owned()])?;
        let [item]: [DecompilationItem; 1] = items
            .try_into()
            .map_err(|_| failure("The pseudocode batch changed its saved address count"))?;
        item.result.map_err(failure)
    }
}
