use super::State;
use crate::comparison::{self, Report};
use crate::error::{Result, failure};
use crate::events::Event;
use serde_json::{Value, json};
use std::path::Path;

impl State {
    pub(super) fn compare_versions(&mut self, args: &Value) -> Result<Value> {
        let path = args["baseline"]
            .as_str()
            .ok_or("A baseline application or project path is required")?;
        self.compare_path(Path::new(path))?;
        self.version_matches(&json!({}))
    }

    pub(super) fn compare_path(&mut self, path: &Path) -> Result<()> {
        let _transition = self.runtime.begin_transition()?;
        let current = self
            .project
            .as_ref()
            .ok_or("Open the destination version first")?
            .clone();
        if !matches!(current.target.kind.as_str(), "app" | "binary") {
            return Err(failure("Version comparison requires native binaries"));
        }
        let baseline = self.read_project(
            path,
            path.extension()
                .is_some_and(|extension| extension == "prism"),
        )?;
        if !matches!(baseline.target.kind.as_str(), "app" | "binary") {
            return Err(failure(
                "The baseline must be a native binary or saved native project",
            ));
        }
        self.comparison = None;
        self.pause_decompilation()?;
        self.close_engine()?;

        let context = crate::decompilation::Context::new(&current.target.sha256, &self.settings)?;
        let cache = comparison::SignatureCache {
            namespace: format!(
                "version-signatures-v2:{}",
                super::analysis_cache_identity::namespace(&context)
            ),
            budget: self.settings.cache_mib.saturating_mul(1024 * 1024),
            listing_only: self.settings.analysis_backend().retains_prepared_session(),
        };

        // Keep only one provider alive at a time; always close the isolated baseline.
        let mut engine = crate::analysis::Engine::open(
            &self.settings,
            &baseline.target.path,
            self.cancelled.clone(),
            self.sink.clone(),
        )?;
        let source = comparison::Capture {
            engine: &mut engine,
            store: &mut self.store,
            cancelled: &self.cancelled,
            sink: &self.sink,
            cache: &cache,
        }
        .run(baseline);
        let source =
            crate::cleanup::finish(source, engine.close(), "Closing the baseline analysis")?;

        self.prepare_engine()?;
        let target = comparison::Capture {
            engine: self
                .engine
                .as_mut()
                .ok_or("The destination engine did not start")?,
            store: &mut self.store,
            cancelled: &self.cancelled,
            sink: &self.sink,
            cache: &cache,
        }
        .run(current)?;
        let report = std::sync::Arc::new(comparison::compare(source, target)?);
        self.sink.send(Event::Comparison(report.clone()))?;
        self.comparison = Some(report);
        Ok(())
    }

    pub(super) fn version_matches(&self, args: &Value) -> Result<Value> {
        let report = self
            .comparison
            .as_ref()
            .ok_or("Run compare_versions first")?;
        let kind = args["kind"].as_str().unwrap_or("matches");
        let offset = usize::try_from(args["offset"].as_u64().unwrap_or(0))?;
        let rows = match kind {
            "matches" => page(&report.matches, offset)?,
            "ambiguous" => return ambiguity_page(report, offset, args),
            "source_only" => page(&report.source_only, offset)?,
            "target_only" => page(&report.target_only, offset)?,
            "source_unknown" => page(&report.source_unknown, offset)?,
            "target_unknown" => page(&report.target_unknown, offset)?,
            _ => return Err(failure("Unknown comparison result kind")),
        };
        let annotations: Vec<_> = if kind == "matches" {
            report
                .matches
                .iter()
                .skip(offset)
                .take(100)
                .map(|item| annotation(report, &item.source.address))
                .collect()
        } else {
            Vec::new()
        };
        Ok(
            json!({"summary":report.summary(),"kind":kind,"offset":offset,
            "page_size":100,"rows":rows,"source_annotations":annotations}),
        )
    }
}

fn page<T: serde::Serialize>(items: &[T], offset: usize) -> Result<Value> {
    Ok(serde_json::to_value(
        items.iter().skip(offset).take(100).collect::<Vec<_>>(),
    )?)
}

pub(super) fn text_report(report: &Report) -> Result<String> {
    Ok(serde_json::to_string_pretty(&report.summary())?)
}

fn ambiguity_page(report: &Report, offset: usize, args: &Value) -> Result<Value> {
    let source_offset = usize::try_from(args["source_offset"].as_u64().unwrap_or(0))?;
    let target_offset = usize::try_from(args["target_offset"].as_u64().unwrap_or(0))?;
    let group = report.ambiguous.get(offset);
    let row = group
        .map(|group| -> Result<Value> {
            let annotations: Vec<_> = group
                .source
                .iter()
                .skip(source_offset)
                .take(100)
                .map(|function| annotation(report, &function.address))
                .collect();
            Ok(json!({
                "group_index": offset,
                "source_count": group.source.len(),
                "target_count": group.target.len(),
                "source_offset": source_offset,
                "target_offset": target_offset,
                "source": page(&group.source, source_offset)?,
                "target": page(&group.target, target_offset)?,
                "source_annotations": annotations,
            }))
        })
        .transpose()?;
    Ok(
        json!({"summary":report.summary(),"kind":"ambiguous","offset":offset,"page_size":1,
        "rows":row.into_iter().collect::<Vec<_>>() }),
    )
}

fn annotation(report: &Report, address: &str) -> Value {
    let note = report.annotations.notes.get(address);
    json!({"address":address,"confirmed_label":report.annotations.labels.get(address),
        "note_preview":note.map(|note| note.chars().take(2048).collect::<String>()),
        "note_truncated":note.is_some_and(|note| note.chars().count()>2048),
        "bookmark":report.annotations.bookmarks.iter().any(|item|item==address)})
}
