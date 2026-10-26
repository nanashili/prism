use super::State;
use super::analysis_cache::ViewCachePolicy;
use crate::analysis::semantic::{Document, Query};
use crate::error::{Result, failure};
use crate::model::{Procedure, View};
use crate::storage::function_search::SearchRequest;
use serde_json::{Value, json};

impl State {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn index_function_search(&mut self, args: &Value) -> Result<Value> {
        let context = self.function_search_context()?;
        let scope = search_scope(&context);
        if args["reset"].as_bool().unwrap_or(false) {
            if args.get("offset").is_some() {
                return Err(failure("Choose reset or an indexing offset"));
            }
            let removed = self
                .store
                .clear_function_search(&context.project, &self.cancelled)?;
            return Ok(
                json!({"removed": removed, "scope": "all search configurations for this binary"}),
            );
        }
        let offset = usize::try_from(args["offset"].as_u64().unwrap_or_default())?;
        let namespace = super::analysis_cache_identity::namespace(&context);
        let page = self.function_search_page(&context, &namespace, offset)?;
        self.store
            .index_search_documents(&scope, &page.documents, &self.cancelled)?;
        Ok(json!({
            "processed": page.documents.len(),
            "coverage": self.store.function_search_coverage(&scope, &context.project)?,
            "with_pseudocode": page.documents.iter().filter(|document| !document.decompiler.is_empty()).count(),
            "next_offset": page.next_offset,
            "evidence": "Saved evidence only. Run decompile_batch and refresh these pages to add missing pseudocode."
        }))
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    fn function_search_page(
        &self,
        context: &crate::decompilation::Context,
        namespace: &str,
        offset: usize,
    ) -> Result<SearchPage> {
        let functions = self.store.functions(&context.project, "", offset)?;
        let mut documents = Vec::new();
        let mut page_bytes = 0;
        for function in &functions {
            if self.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(failure("Function search indexing cancelled"));
            }
            let document = self.search_evidence(context, namespace, function)?;
            document
                .validate()
                .map_err(|error| failure(format!("Cannot index {}: {error}", function.address)))?;
            page_bytes += serde_json::to_vec(&document)?.len();
            documents.push(document);
            if page_bytes >= 8 * 1024 * 1024 {
                break;
            }
        }
        let next_offset = (documents.len() < functions.len() || functions.len() == 300)
            .then_some(offset + documents.len());
        Ok(SearchPage {
            documents,
            next_offset,
        })
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn semantic_search(&self, args: &Value) -> Result<Value> {
        let context = self.function_search_context()?;
        let scope = search_scope(&context);
        let query_text = args["query"]
            .as_str()
            .ok_or("A behavior query is required")?;
        let query = Query::parse(query_text)?;
        let limit = args["limit"].as_u64().unwrap_or(20);
        if !(1..=100).contains(&limit) {
            return Err(failure("Use a search result limit of 1–100"));
        }
        let exemplar = args["similar_to"]
            .as_str()
            .map(|text| {
                crate::analysis::graph::address_value(text)
                    .map(|number| format!("0x{number:x}"))
                    .ok_or("The exemplar must be a native function address")
            })
            .transpose()?;
        let mut hits = self.store.search_functions(&SearchRequest {
            scope: &scope,
            project: &context.project,
            query: &query,
            limit: usize::try_from(limit)?,
            similar_to: exemplar.as_deref(),
            cancelled: &self.cancelled,
        })?;
        let project = self.project.as_ref().ok_or("Open a file first")?;
        for hit in &mut hits {
            if let Some(name) = project.renamed.get(&hit.address) {
                hit.name.clone_from(name);
            }
        }
        Ok(json!({
            "query": query_text, "matches": hits,
            "coverage": self.store.function_search_coverage(&scope, &context.project)?,
            "candidate_limit_per_route": crate::storage::function_search::CANDIDATE_LIMIT,
            "method": "Native Rust Ghidra saved-evidence term/synonym retrieval and inferred behavior, expanded by normalized pseudocode/CFG feature cosine; no Java, BSim signatures or learned text embeddings.",
            "limitations": "Scores rank leads, not verified behavior or probabilities. Structural similarity transfers a lead from the named exemplar; it does not prove the query. Only indexed functions in the current saved inventory/configuration are searched. Refresh pages after evidence or annotations change."
        }))
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    fn function_search_context(&self) -> Result<crate::decompilation::Context> {
        let project = self.project.as_ref().ok_or("Open a file first")?;
        if project.target.kind == "android"
            || !self.settings.analysis_backend().retains_prepared_session()
        {
            return Err(failure(
                "Function behavior indexing requires the native Rust Ghidra backend",
            ));
        }
        self.decompilation_context()
    }

    fn search_evidence(
        &self,
        context: &crate::decompilation::Context,
        namespace: &str,
        function: &Procedure,
    ) -> Result<Document> {
        let mut document = Document::default();
        document.address.clone_from(&function.address);
        let open_project = self.project.as_ref().ok_or("Open a file first")?;
        document.name.clone_from(
            open_project
                .renamed
                .get(&function.address)
                .unwrap_or(&function.name),
        );
        document.notes = open_project
            .notes
            .get(&function.address)
            .cloned()
            .unwrap_or_default();
        if let Some(code) =
            self.saved_search_view(context, namespace, &function.address, View::Pseudocode)?
        {
            document.decompiler = code;
            document.constants = constants(&document.decompiler);
        }
        if let Some(raw) =
            self.saved_search_view(context, namespace, &function.address, View::References)?
        {
            let references: crate::analysis::references::References = serde_json::from_str(&raw)?;
            document.imports = references
                .outgoing
                .into_iter()
                .map(|reference| reference.name.unwrap_or(reference.address))
                .collect();
            document.callers = references
                .incoming
                .into_iter()
                .map(|reference| reference.name.unwrap_or(reference.address))
                .collect();
        }
        let graph = self
            .saved_search_view(context, namespace, &function.address, View::Graph)?
            .map(|raw| serde_json::from_str::<crate::analysis::graph::Graph>(&raw))
            .transpose()?;
        document.control_flow = graph.as_ref().map(graph_features).unwrap_or_default();
        document.structure =
            crate::analysis::semantic::structure::features(&document.decompiler, graph.as_ref());
        document.structure_space = Some(crate::analysis::semantic::structure::SPACE.into());
        Ok(document)
    }

    fn saved_search_view(
        &self,
        context: &crate::decompilation::Context,
        namespace: &str,
        address: &str,
        view: View,
    ) -> Result<Option<String>> {
        let key = format!("{}:{}", namespace, view.label());
        self.cached_view(
            &context.project,
            address,
            &key,
            view,
            ViewCachePolicy::Strict,
        )
    }
}

struct SearchPage {
    documents: Vec<Document>,
    next_offset: Option<usize>,
}

fn search_scope(context: &crate::decompilation::Context) -> String {
    format!(
        "function-search-v1:{}:{}",
        context.project,
        super::analysis_cache_identity::namespace(context)
    )
}

fn constants(code: &str) -> Vec<String> {
    code.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|word| {
            word.len() >= 2
                && word.len() <= 32
                && word.as_bytes().first().is_some_and(u8::is_ascii_digit)
        })
        .take(256)
        .map(str::to_owned)
        .collect()
}

fn graph_features(graph: &crate::analysis::graph::Graph) -> Vec<String> {
    let mut features = vec![format!("{} basic blocks", graph.blocks.len())];
    if graph.blocks.iter().any(|block| block.successors.len() > 1) {
        features.push("conditional branch".into());
    }
    let back_edge = graph.blocks.iter().any(|block| {
        let start = crate::analysis::graph::address_value(&block.start);
        block.successors.iter().any(|target| {
            matches!((crate::analysis::graph::address_value(target), start), (Some(target), Some(start)) if target <= start)
        })
    });
    if back_edge {
        features.push("loop back edge (inferred)".into());
    }
    features
}
