use super::State;
use super::analysis_cache::ViewCachePolicy;
use crate::analysis::{Engine, local};
use crate::error::Result;
use crate::events::Event;
use crate::model::View;
use crate::project::files;
use serde_json::Value;

#[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
mod batch;
mod view;

impl State {
    pub(super) fn discard_unusable_engine(&mut self) {
        if self
            .engine
            .as_ref()
            .is_some_and(|engine| !engine.is_reusable())
        {
            self.engine = None;
        }
    }

    pub(super) fn engine(&mut self) -> Result<&mut Engine> {
        self.discard_unusable_engine();

        if self.engine.is_none() && self.settings.analysis_backend().retains_prepared_session() {
            return Err(crate::error::failure(
                "The Ghidra session is not prepared. Reopen the project or choose Analyze first. Decompilation will not start a new analysis automatically.",
            ));
        }

        self.prepare_engine()
    }

    pub(super) fn analysis_call(&mut self, name: &str, arguments: Value) -> Result<Value> {
        Self::reject_direct_decompilation_tool(name)?;
        // Unknown provider operations may mutate analysis even when they fail.
        if !matches!(
            name,
            "procedure_assembly"
                | "analyze_function"
                | "procedure_callers"
                | "procedure_callees"
                | "list_procedures"
                | "list_segments"
                | "list_strings"
                | "inspect_analysis_view"
        ) {
            let project = self.project.as_ref().ok_or("Open a file first")?;
            self.store.invalidate_verification(&project.target.sha256)?;
        }
        self.engine()?.tool(name, arguments)
    }

    pub(super) fn reject_direct_decompilation_tool(name: &str) -> Result<()> {
        if matches!(name, "procedure_pseudo_code" | "batch_decompile") {
            return Err(crate::error::failure(
                "Use Prism's saved-inventory decompilation batch for pseudocode",
            ));
        }
        Ok(())
    }

    /// Only explicit project opening or analysis may prepare a new external session.
    pub(super) fn prepare_engine(&mut self) -> Result<&mut Engine> {
        self.discard_unusable_engine();
        let target = &self.project.as_ref().ok_or("Open a file first")?.target;
        if target.kind == "android" {
            return Err(crate::error::failure(
                "Android analysis uses jadx. REA tools are available for native binaries.",
            ));
        }
        if self.engine.is_none() {
            self.sink.send(Event::Progress(format!(
                "Starting {}",
                self.settings.provider
            )))?;
            let target = &self.project.as_ref().ok_or("Open a file first")?.target;
            let engine = Engine::open(
                &self.settings,
                &target.path,
                self.cancelled.clone(),
                self.sink.clone(),
            )?;
            self.engine = Some(engine);
        }
        self.engine
            .as_mut()
            .ok_or_else(|| crate::error::failure("The analysis engine did not start"))
    }

    pub(super) fn content(
        &mut self,
        view: View,
        address: String,
        offset: u64,
        request: Option<u64>,
    ) -> Result<String> {
        let text = self.read_view(view, &address, offset)?;
        self.sink.send(Event::Content {
            request,
            view,
            address,
            text,
        })?;
        Ok(String::new())
    }

    pub(super) fn read_view(&mut self, view: View, address: &str, offset: u64) -> Result<String> {
        let target = &self.project.as_ref().ok_or("Open a file first")?.target;
        if view == View::Pseudocode
            && !address.is_empty()
            && matches!(target.kind.as_str(), "app" | "binary")
        {
            self.require_indexed_function(address)?;
        }
        self.read_view_with_cache(view, address, offset, ViewCachePolicy::Interactive)
    }

    /// Batch callers validate against their current or frozen inventory before reading.
    pub(super) fn read_view_strict(
        &mut self,
        view: View,
        address: &str,
        offset: u64,
    ) -> Result<String> {
        self.read_view_with_cache(view, address, offset, ViewCachePolicy::Strict)
    }

    pub(super) fn read_durable_pseudocode(&mut self, address: &str) -> Result<String> {
        self.read_view_with_cache(View::Pseudocode, address, 0, ViewCachePolicy::DurableResult)
    }

    fn read_view_with_cache(
        &mut self,
        view: View,
        address: &str,
        offset: u64,
        policy: ViewCachePolicy,
    ) -> Result<String> {
        let project = self.project.as_ref().ok_or("Open a file first")?;
        let target = project.target.clone();
        let context = self.decompilation_context()?;
        let provider = crate::worker::analysis_cache_identity::namespace(&context);
        let key = format!("{provider}:{}", view.label());
        let use_bounded_batch_preview = view == View::Pseudocode
            && !address.is_empty()
            && matches!(target.kind.as_str(), "app" | "binary")
            && policy == ViewCachePolicy::Interactive
            && !self.settings.analysis_backend().retains_prepared_session();
        let cached = if use_bounded_batch_preview {
            None
        } else {
            self.cached_view(&target.sha256, address, &key, view, policy)?
        };

        if self.settings.analysis_backend().retains_prepared_session()
            && matches!(target.kind.as_str(), "app" | "binary")
            && view == View::Pseudocode
            && policy != ViewCachePolicy::Interactive
            && cached.is_none()
        {
            return Err(crate::error::failure(
                "Run decompile_batch with action:start for these indexed addresses first. Strict pseudocode reads reuse verified Ghidra batch results.",
            ));
        }

        let mut text = match view {
            View::Assembly if address.is_empty() && target.kind != "android" => {
                self.whole_listing(offset)?
            }
            View::Hex => files::hex(&target.path, offset)?,
            View::Strings => match cached {
                Some(text) => text,
                None => local::strings(&target.path, &self.cancelled)?,
            },
            View::Imports => crate::analysis::file_inventory::read(&target)?.text,
            View::Resources => match address.strip_prefix("resource:") {
                Some(relative) => crate::analysis::resources::read(&target, relative)?,
                None => local::resource_list(&target.path, &self.cancelled)?,
            },
            View::Notes => project.notes.get(address).cloned().unwrap_or_default(),
            View::Pseudocode if target.kind == "source" => files::text(&target.path)?,
            View::Pseudocode if target.kind == "model" => local::model(&target.path)?,
            View::Pseudocode if target.kind == "android" => {
                crate::analysis::android::code(&target, &self.store, address)?
            }
            View::Assembly | View::Graph | View::References if target.kind == "android" => {
                return Err(crate::error::failure(
                    "Android bytecode and method graphs are not available in this Rust build yet. Java methods are in Pseudocode.",
                ));
            }
            View::Pseudocode
                if target.kind == "gpu"
                    && target
                        .path
                        .extension()
                        .is_some_and(|extension| extension == "ptx") =>
            {
                files::text(&target.path)?
            }
            View::Diff | View::Debug | View::Reconstruct => return Ok(String::new()),
            _ => match cached {
                Some(text) => text,
                None => {
                    let text = if use_bounded_batch_preview {
                        self.single_address_decompile_batch(address)?
                    } else {
                        self.engine_view(view, address)?
                    };
                    if policy.retains_views() && !use_bounded_batch_preview {
                        self.store.cache_view(
                            &target.sha256,
                            address,
                            &key,
                            &text,
                            self.settings.cache_mib.saturating_mul(1024 * 1024),
                        )?;
                    }
                    text
                }
            },
        };
        // Persist only static graphs and reapply current evidence after every cache read.
        if view == View::Graph {
            let mut graph: crate::analysis::graph::Graph = serde_json::from_str(&text)?;
            let traces = self.store.execution_traces(&target.sha256)?;
            graph.annotate(&traces);
            text = serde_json::to_string(&graph)?;
        }
        Ok(text)
    }
}
