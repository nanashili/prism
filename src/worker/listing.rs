use super::State;
use crate::error::Result;
use crate::model::View;
use crate::storage::navigation::{Query, Row, Sort};
use std::fmt::Write;

impl State {
    pub(super) fn whole_listing(&mut self, offset: u64) -> Result<String> {
        let project = self.project.as_ref().ok_or("Open a file first")?;
        let query = Query {
            sort: Sort::Address,
            offset: usize::try_from(offset)?,
            renamed: project.renamed.clone(),
            ..Default::default()
        };
        let page = self
            .store
            .navigation_page(&project.target.sha256, &query, &self.cancelled)?;
        if page.rows.is_empty() {
            return Ok(
                "No functions at this position. Analyze the file to populate the listing.".into(),
            );
        }

        let mut listing = String::new();
        for row in page.rows.into_iter().take(5) {
            if self.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(crate::error::failure("Listing cancelled"));
            }
            let Row::Function { procedure, .. } = row else {
                continue;
            };
            writeln!(listing, "{}:  {}\n", procedure.name, procedure.address)?;
            match self.read_view(View::Assembly, &procedure.address, 0) {
                Ok(text) => listing.push_str(&text),
                Err(error) => writeln!(listing, "Could not read this function: {error}")?,
            }
            listing.push_str("\n\n");
            if listing.len() >= crate::project::files::TEXT_LIMIT {
                let mut end = crate::project::files::TEXT_LIMIT;
                while !listing.is_char_boundary(end) {
                    end -= 1;
                }
                listing.truncate(end);
                listing.push_str(
                    "\nDisplay limited to 1 MiB. Open the function separately for its full view.\n",
                );
                break;
            }
        }
        Ok(listing)
    }
}
