use super::State;
use crate::error::Result;
use crate::events::Event;
use crate::storage::navigation::{Counts, Query};

impl State {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn resource_page(
        &mut self,
        project: String,
        generation: u64,
        parent: String,
        offset: usize,
    ) -> Result<String> {
        let Some(current) = self
            .project
            .as_ref()
            .filter(|current| current.target.sha256 == project)
        else {
            return Ok(String::new());
        };
        let root = crate::analysis::resources::root(&current.target);
        let page = self
            .store
            .resource_page(root.as_deref(), &parent, offset, &self.cancelled)
            .map_err(|error| error.to_string());
        self.sink.send(Event::Resources {
            project,
            generation,
            parent,
            offset,
            page,
        })?;
        Ok(String::new())
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn sidebar(
        &mut self,
        project: String,
        generation: u64,
        query: Query,
    ) -> Result<String> {
        if self
            .project
            .as_ref()
            .map(|project| project.target.sha256.as_str())
            != Some(&project)
        {
            return Ok(String::new());
        }
        let mut page = self
            .store
            .navigation_page(&project, &query, &self.cancelled)
            .map_err(|error| error.to_string());
        if query.parent.is_empty() {
            page = page.and_then(|mut page| {
                page.counts = self
                    .sidebar_inventory(&project)
                    .map_err(|error| error.to_string())?;
                Ok(page)
            });
        }
        self.sink.send(Event::Sidebar {
            project,
            generation,
            parent: query.parent,
            offset: query.offset,
            page,
        })?;
        Ok(String::new())
    }

    fn sidebar_inventory(&mut self, project: &str) -> Result<Counts> {
        if let Some((identity, counts)) = &self.sidebar_counts
            && identity == project
        {
            return Ok(counts.clone());
        }
        let target = &self.project.as_ref().ok_or("Open a file first")?.target;
        let mut counts = Counts::default();
        if matches!(target.kind.as_str(), "binary" | "app") {
            match crate::analysis::file_inventory::read(target) {
                Ok(inventory) => {
                    counts.imports = Some(inventory.imports);
                    counts.segments = Some(inventory.sections);
                    counts.format = inventory.format;
                }
                Err(error) => counts.warning = Some(error.to_string()),
            }
        }
        self.sidebar_counts = Some((project.into(), counts.clone()));
        Ok(counts)
    }
}
