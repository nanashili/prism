use crate::storage::navigation::{Page, Query};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

#[derive(Default)]
pub(in crate::ui) struct Navigation {
    pub(super) resources: super::resources::Resources,
    pub(super) generation: u64,
    pub(super) query: Query,
    pub(super) pages: BTreeMap<String, LoadedPage>,
    pub(super) expanded: BTreeSet<String>,
    pub(super) closed: BTreeSet<&'static str>,
    pub(super) work: Option<&'static str>,
    pub(super) work_offset: usize,
    pub(super) debounce: Option<Instant>,
    pub(in crate::ui) hidden: bool,
}

pub(super) struct LoadedPage {
    pub page: Page,
    pub offset: usize,
    pub ready: bool,
    pub error: Option<String>,
}

impl Navigation {
    pub(super) fn discard_children(&mut self, parent: &str) {
        let prefix = format!("{}/", parent.trim_end_matches('/'));
        self.expanded
            .retain(|key| !parent.is_empty() && (key == parent || !key.starts_with(&prefix)));
        self.pages
            .retain(|key, _| key == parent || (!parent.is_empty() && !key.starts_with(&prefix)));
    }

    pub(in crate::ui) fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pages.clear();
        self.expanded.clear();
    }

    pub(in crate::ui) fn total(&self) -> usize {
        self.pages.get("").map_or(0, |loaded| loaded.page.total)
    }

    pub(in crate::ui) fn counts(&self) -> Option<&crate::storage::navigation::Counts> {
        self.pages.get("").map(|loaded| &loaded.page.counts)
    }
}
