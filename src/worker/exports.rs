use super::{AnalysisExport, State};
use crate::error::{Result, failure};
use crate::model::View;
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::Ordering;

impl State {
    pub(super) fn export_analysis(&mut self, path: &Path, kind: AnalysisExport) -> Result<String> {
        let project = self.project.as_ref().ok_or("Open a file first")?;
        crate::project::files::preserve_original(&project.target.path, path)?;

        crate::storage::atomic::write(path, |file| {
            self.check_export_cancelled()?;
            match kind {
                AnalysisExport::Function { view, address } => {
                    if address.is_empty() {
                        return Err(failure("Select a function to export"));
                    }
                    file.write_all(self.read_view(view, &address, 0)?.as_bytes())?;
                }
                AnalysisExport::AllPseudocode => self.write_all_pseudocode(file)?,
                AnalysisExport::ObjectiveC => self.write_objc_inventory(file)?,
            }
            self.check_export_cancelled()
        })?;
        self.sink
            .send(crate::events::Event::Exported(path.to_path_buf()))?;
        Ok(format!("Exported to {}", path.display()))
    }

    fn write_all_pseudocode(&mut self, file: &mut std::fs::File) -> Result<()> {
        let project = self.project.as_ref().ok_or("Open a file first")?;
        let id = project.target.sha256.clone();
        writeln!(file, "// Pseudocode for {}\n", project.target.name)?;
        let mut offset = 0;
        let mut failed = 0;

        loop {
            let rows = self.store.functions(&id, "", offset)?;
            if rows.is_empty() {
                break;
            }
            for procedure in &rows {
                self.check_export_cancelled()?;
                writeln!(file, "// {} ({})", procedure.name, procedure.address)?;
                match self.read_view(View::Pseudocode, &procedure.address, 0) {
                    Ok(text) => writeln!(file, "{text}\n")?,
                    Err(error) => {
                        self.check_export_cancelled()?;
                        failed += 1;
                        writeln!(file, "// FAILED: {error}\n")?;
                    }
                }
            }
            offset += rows.len();
            self.sink.send(crate::events::Event::Progress(format!(
                "Exported {offset} functions ({failed} failed)"
            )))?;
        }

        if offset == 0 {
            return Err(failure("Analyze the file before exporting all pseudocode"));
        }
        writeln!(file, "// Completed: {offset} functions; {failed} failed.")?;
        Ok(())
    }

    fn check_export_cancelled(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        Ok(())
    }

    fn write_objc_inventory(&mut self, file: &mut std::fs::File) -> Result<()> {
        let name = &self
            .project
            .as_ref()
            .ok_or("Open a file first")?
            .target
            .name;
        writeln!(file, "// Objective-C in {name}\n")?;
        for (tool, arguments, key, title) in [
            (
                "get_objc_classes",
                json!({"pattern": ""}),
                "classes",
                "Classes",
            ),
            ("get_objc_protocols", json!({}), "protocols", "Protocols"),
        ] {
            self.check_export_cancelled()?;
            let reply = self.engine()?.normalized(tool, arguments)?;
            write_inventory_section(file, &reply, key, title)?;
        }
        Ok(())
    }
}

fn write_inventory_section(
    file: &mut std::fs::File,
    reply: &Value,
    key: &str,
    title: &str,
) -> Result<()> {
    let rows = reply[key]
        .as_array()
        .ok_or("The engine did not return an Objective-C inventory")?;
    writeln!(file, "{title}")?;
    if rows.is_empty() {
        writeln!(file, "  none")?;
    }
    for row in rows {
        let address = row["address"].as_str().ok_or("An entry has no address")?;
        let name = row["name"].as_str().ok_or("An entry has no name")?;
        writeln!(file, "  {address}  {name}")?;
    }
    writeln!(file)?;
    Ok(())
}
