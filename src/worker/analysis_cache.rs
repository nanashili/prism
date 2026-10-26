use super::State;
use crate::error::Result;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewCachePolicy {
    Interactive,
    Strict,
    DurableResult,
}

impl ViewCachePolicy {
    pub(super) fn retains_views(self) -> bool {
        self == Self::Interactive
    }
}

impl State {
    pub(super) fn require_indexed_function(&self, address: &str) -> Result<()> {
        let project = &self
            .project
            .as_ref()
            .ok_or("Open a file first")?
            .target
            .sha256;
        if self.store.has_function(project, address)? {
            return Ok(());
        }
        let number = crate::analysis::graph::address_value(address)
            .ok_or("Choose an analysed function address")?;
        let canonical = format!("0x{number:x}");
        if canonical != address && self.store.has_function(project, &canonical)? {
            return Ok(());
        }
        Err(crate::error::failure(format!(
            "Function {address} is not in the saved analysis. Select an indexed function. Decompilation does not discover functions."
        )))
    }

    pub(super) fn cached_view(
        &self,
        project: &str,
        address: &str,
        key: &str,
        view: crate::model::View,
        policy: ViewCachePolicy,
    ) -> Result<Option<String>> {
        if policy != ViewCachePolicy::Interactive
            && let Some(text) = self.cached_durable_pseudocode(project, address, view)?
        {
            return Ok(Some(text));
        }
        if self.settings.analysis_backend().retains_prepared_session()
            && view == crate::model::View::Pseudocode
            && policy != ViewCachePolicy::Interactive
        {
            return Ok(None);
        }
        if let Some(text) = self.store.cache(project, address, key)? {
            return Ok(Some(text));
        }
        if policy == ViewCachePolicy::Interactive {
            if let Some(text) = self.cached_durable_pseudocode(project, address, view)? {
                return Ok(Some(text));
            }
            if self.settings.analysis_backend().retains_prepared_session() {
                return Ok(None);
            }
        }
        Ok(None)
    }

    pub(super) fn cache_accepted_batch_pseudocode(
        &self,
        address: &str,
        pseudocode: &str,
    ) -> Result<()> {
        let project = &self.project.as_ref().ok_or("Open a file first")?.target;
        let context = self.decompilation_context()?;
        let key = format!(
            "{}:{}",
            crate::worker::analysis_cache_identity::namespace(&context),
            crate::model::View::Pseudocode.label()
        );
        self.store.cache_view(
            &project.sha256,
            address,
            &key,
            pseudocode,
            self.settings.cache_mib.saturating_mul(1024 * 1024),
        )
    }

    fn cached_durable_pseudocode(
        &self,
        project: &str,
        address: &str,
        view: crate::model::View,
    ) -> Result<Option<String>> {
        if view != crate::model::View::Pseudocode || address.is_empty() {
            return Ok(None);
        }
        let context = self.decompilation_context()?;
        let canonical =
            crate::analysis::graph::address_value(address).map(|number| format!("0x{number:x}"));
        self.store.durable_pseudocode(
            project,
            &context.provider,
            &context.configuration,
            canonical.as_deref().unwrap_or(address),
        )
    }
}
