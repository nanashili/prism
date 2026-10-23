use super::Funcdata;
use crate::arena::{Arena, ArenaId};

/// Numeric storage observations; allocated IDs are never reused.
#[derive(Clone, Copy, Debug)]
pub struct ArenaStorageCounts {
    pub live: usize,
    pub allocated: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct AnalysisStorageCounts {
    pub operations: ArenaStorageCounts,
    pub varnodes: ArenaStorageCounts,
    pub blocks: ArenaStorageCounts,
    pub highs: ArenaStorageCounts,
    pub groups: ArenaStorageCounts,
    pub calls: ArenaStorageCounts,
}

impl Funcdata {
    /// Observe retained arena slots without changing analysis or ID lifetimes.
    pub fn analysis_storage_counts(&self) -> AnalysisStorageCounts {
        AnalysisStorageCounts {
            operations: count(&self.obank.ops),
            varnodes: count(&self.vbank.varnodes),
            blocks: count(&self.blocks),
            highs: count(&self.highs),
            groups: count(&self.groups),
            calls: count(&self.callspecs),
        }
    }
}

fn count<Id: ArenaId, T>(arena: &Arena<Id, T>) -> ArenaStorageCounts {
    ArenaStorageCounts {
        live: arena.len(),
        allocated: arena.capacity_ids(),
    }
}
