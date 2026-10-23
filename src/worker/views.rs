use super::{State, ViewJob};
use crate::error::{Result, failure};
use crate::events::Event;
use crate::model::View;
use crate::project::files;

impl State {
    pub(super) fn execute_view(&mut self, request: u64, job: ViewJob) -> Result<String> {
        match job {
            ViewJob::Content {
                view,
                address,
                offset,
            } => self.content(view, address, offset, Some(request)),
            ViewJob::Tool { name, args } => self.execute_tool_view(request, &name, args),
            ViewJob::Diff(path) => {
                let text = self.diff(&path)?;
                self.send_view(request, View::Diff, text)?;
                Ok("Comparison complete".into())
            }
            ViewJob::Debug(action) => {
                let text = crate::debug::execute(
                    &mut self.debugger,
                    action,
                    &self.settings,
                    self.cancelled.clone(),
                    self.sink.clone(),
                )?;
                self.send_view(request, View::Debug, text)?;
                Ok(String::new())
            }
        }
    }

    fn execute_tool_view(
        &mut self,
        request: u64,
        name: &str,
        args: serde_json::Value,
    ) -> Result<String> {
        let text = if name == "procedure_pseudo_code" {
            let address = args["procedure"]
                .as_str()
                .ok_or("A function address is required")?;
            self.decompile_tool_address(address)?
        } else {
            Self::reject_direct_decompilation_tool(name)?;
            let result = self.engine()?.normalized(name, args)?;
            serde_json::to_string_pretty(&result)?
        };

        self.send_view(request, View::Pseudocode, text)?;
        Ok("Tool completed".into())
    }

    fn decompile_tool_address(&mut self, address: &str) -> Result<String> {
        #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
        if self.settings.analysis_backend().retains_prepared_session() {
            return self.batch_pseudocode(address);
        }

        self.single_address_decompile_batch(address)
    }

    fn send_view(&self, request: u64, view: View, text: String) -> Result<()> {
        self.sink.send(Event::Content {
            request: Some(request),
            view,
            address: String::new(),
            text,
        })
    }

    fn diff(&self, path: &std::path::Path) -> Result<String> {
        use std::fmt::Write;
        use std::io::Read;
        let project = self.project.as_ref().ok_or("Open a file first")?;
        let mut left = std::fs::File::open(&project.target.path)?;
        let mut right = std::fs::File::open(path)?;
        let mut before = [0u8; 4096];
        let mut after = [0u8; 4096];
        let mut report = format!(
            "Byte changes against {}\nOffsets are file offsets.\n\n",
            path.display()
        );
        let mut offset = 0;
        loop {
            if self.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            let a = left.read(&mut before)?;
            let b = right.read(&mut after)?;
            if a == 0 && b == 0 {
                break;
            }
            if before[..a] != after[..b] {
                writeln!(report, "0x{offset:012x}: changed block ({a} / {b} bytes)")?;
            }
            offset += 4096;
            if report.len() > files::TEXT_LIMIT {
                report.push_str("\nDisplay limit reached\n");
                break;
            }
        }
        Ok(report)
    }
}
