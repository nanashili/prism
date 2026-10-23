use super::types::Report;
use crate::error::Result;
use std::path::Path;

pub(super) fn import(root: &Path, destination: &Path, report: &mut Report) -> Result<()> {
    let settings = crate::settings::Settings::load()?;
    let skills = if settings.desktop.skills_dir.is_empty() {
        destination.join("skills")
    } else {
        std::path::PathBuf::from(&settings.desktop.skills_dir)
    };
    merge(&root.join("support/skills"), &skills)?;
    crate::agent::picks::import_from(
        &root.join("support/agent-picks.json"),
        &destination.join("agent-picks.json"),
    )?;
    crate::agent::models::import_from(
        &root.join("support/agent-models.json"),
        &destination.join("agent-models.json"),
    )?;
    merge(
        &root.join("support/analysis/ghidra-projects"),
        &destination.join("ghidra"),
    )?;
    report.warnings.push(
        "Thoughts and tool names/timings are restored. Full tool payloads, permissions and subagent records remain in the original compressed chats.".into(),
    );
    if root.join("support/reconstruction.sqlite.zst").is_file() {
        report.warnings.push(
            "The consistent reconstruction database backup is support/reconstruction.sqlite.zst; resuming its jobs is not implemented in the Rust app.".into(),
        );
    }
    Ok(())
}

fn merge(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(destination)?;
    for item in std::fs::read_dir(source)? {
        let item = item?;
        let target = destination.join(item.file_name());
        if item.file_type()?.is_dir() {
            merge(&item.path(), &target)?;
        } else if item.file_type()?.is_file() && !target.exists() {
            super::snapshot::copy_file(&item.path(), &target)?;
        }
    }
    Ok(())
}
