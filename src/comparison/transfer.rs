use super::{Report, Selection};
use crate::error::{Result, failure};
use crate::model::Project;
use std::collections::{BTreeMap, BTreeSet};

/// Prepare the complete edit before the caller commits it through project persistence/undo.
pub(crate) fn transfer(
    report: &Report,
    destination: &Project,
    selections: &[Selection],
) -> Result<Project> {
    if destination.target.sha256 != report.target_sha256 {
        return Err(failure(
            "This comparison belongs to a different destination binary",
        ));
    }
    if selections.is_empty() {
        return Err(failure("Select at least one reviewed function match"));
    }
    validate_selections(report, destination, selections)?;
    let mut updated = destination.clone();
    copy_annotations(report, &mut updated, selections);
    record_transfer(report, &mut updated, selections)?;
    Ok(updated)
}

fn validate_selections(
    report: &Report,
    destination: &Project,
    selections: &[Selection],
) -> Result<()> {
    let candidates = Candidates::new(report);
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for selection in selections {
        if !candidates.permits(selection) {
            return Err(failure(
                "A selected pair is not a candidate in this comparison",
            ));
        }
        if !sources.insert(&selection.source) || !targets.insert(&selection.target) {
            return Err(failure("Reviewed matches must be one-to-one"));
        }
        check_conflict(
            &destination.renamed,
            &report.annotations.labels,
            selection,
            "label",
        )?;
        check_conflict(
            &destination.notes,
            &report.annotations.notes,
            selection,
            "note",
        )?;
    }
    Ok(())
}

fn copy_annotations(report: &Report, updated: &mut Project, selections: &[Selection]) {
    let source_bookmarks: BTreeSet<_> = report.annotations.bookmarks.iter().collect();
    let mut target_bookmarks: BTreeSet<_> = updated.bookmarks.iter().cloned().collect();
    for selection in selections {
        if let Some(label) = report.annotations.labels.get(&selection.source) {
            updated
                .renamed
                .insert(selection.target.clone(), label.clone());
        }
        if let Some(note) = report.annotations.notes.get(&selection.source) {
            updated.notes.insert(selection.target.clone(), note.clone());
        }
        if source_bookmarks.contains(&selection.source)
            && target_bookmarks.insert(selection.target.clone())
        {
            updated.bookmarks.push(selection.target.clone());
        }
    }
}

fn record_transfer(report: &Report, updated: &mut Project, selections: &[Selection]) -> Result<()> {
    let provenance = serde_json::json!({
        "source_sha256":report.source_sha256,"target_sha256":report.target_sha256,
        "reviewed_pairs":selections,
        "scope":"Function labels, notes and bookmarks; discoveries need revalidation in the destination."
    });
    let imported = std::sync::Arc::make_mut(&mut updated.imported);
    let history = imported
        .entry("version-transfers.json".into())
        .or_insert_with(|| serde_json::json!([]));
    let history = history
        .as_array_mut()
        .ok_or("Invalid version transfer history")?;
    if history.len() >= 100 {
        return Err(failure("Version transfer history is limited to 100 edits"));
    }
    history.push(provenance);
    if serde_json::to_vec(history)?.len() > 2 * 1024 * 1024 {
        return Err(failure("Version transfer history exceeds 2 MiB"));
    }
    Ok(())
}

fn check_conflict(
    destination: &std::collections::BTreeMap<String, String>,
    source: &std::collections::BTreeMap<String, String>,
    selection: &Selection,
    kind: &str,
) -> Result<()> {
    if let (Some(existing), Some(incoming)) = (
        destination.get(&selection.target),
        source.get(&selection.source),
    ) && existing != incoming
    {
        return Err(failure(format!(
            "Existing {kind} at {} conflicts with the source; resolve it before transfer",
            selection.target
        )));
    }
    Ok(())
}

struct Candidates<'a> {
    matches: BTreeMap<&'a str, &'a str>,
    source_groups: BTreeMap<&'a str, usize>,
    target_groups: BTreeMap<&'a str, usize>,
}

impl<'a> Candidates<'a> {
    fn new(report: &'a Report) -> Self {
        let matches = report
            .matches
            .iter()
            .map(|item| (item.source.address.as_str(), item.target.address.as_str()))
            .collect();
        let mut source_groups = BTreeMap::new();
        let mut target_groups = BTreeMap::new();
        for (index, group) in report.ambiguous.iter().enumerate() {
            source_groups.extend(
                group
                    .source
                    .iter()
                    .map(|item| (item.address.as_str(), index)),
            );
            target_groups.extend(
                group
                    .target
                    .iter()
                    .map(|item| (item.address.as_str(), index)),
            );
        }
        Self {
            matches,
            source_groups,
            target_groups,
        }
    }

    fn permits(&self, selection: &Selection) -> bool {
        if self.matches.get(selection.source.as_str()).copied() == Some(selection.target.as_str()) {
            return true;
        }
        self.source_groups
            .get(selection.source.as_str())
            .zip(self.target_groups.get(selection.target.as_str()))
            .is_some_and(|(source, target)| source == target)
    }
}
