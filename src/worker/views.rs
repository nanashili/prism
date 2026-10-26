use super::{State, ViewJob};
use crate::error::Result;
use crate::events::Event;
use crate::model::View;

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
                self.compare_path(&path)?;
                let text = super::comparison::text_report(
                    self.comparison
                        .as_ref()
                        .ok_or("Missing comparison report")?,
                )?;
                self.send_view(request, View::Diff, text)?;
                Ok("Comparison complete".into())
            }
            ViewJob::Debug(action) => {
                let text = self.execute_evidence_debug(action)?;
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
            let mut reply = self.analysis_call(name, args)?;
            let result = reply
                .get_mut("normalized_result")
                .map(serde_json::Value::take)
                .unwrap_or(reply);
            serde_json::to_string_pretty(&result)?
        };

        self.send_view(request, View::Pseudocode, text)?;
        Ok("Tool completed".into())
    }

    fn decompile_tool_address(&mut self, address: &str) -> Result<String> {
        #[cfg(feature = "rea-rust-ghidra")]
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
}
