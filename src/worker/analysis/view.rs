use super::State;
use crate::error::Result;
use crate::model::View;
use crate::project::files;
use serde_json::{Value, json};
use std::io::Write;

#[cfg(test)]
mod tests;

impl State {
    pub(super) fn engine_view(&mut self, view: View, address: &str) -> Result<String> {
        if address.is_empty()
            && matches!(
                view,
                View::Pseudocode | View::Assembly | View::Graph | View::References
            )
        {
            return unselected_view(view);
        }
        if view == View::References {
            let dossier = self.read_dossier(address)?;
            let references = crate::analysis::references::References::from_dossier(&dossier)?;
            return Ok(serde_json::to_string(&references)?);
        }
        if view == View::Graph {
            let dossier = self.read_dossier(address)?;
            let assembly = self.read_view(View::Assembly, address, 0)?;
            let graph = crate::analysis::graph::Graph::from_dossier(&dossier, &assembly)?;
            return Ok(serde_json::to_string(&graph)?);
        }
        self.read_engine_text(view, address)
    }

    fn read_engine_text(&mut self, view: View, address: &str) -> Result<String> {
        let (tool, args) = match view {
            #[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
            View::Pseudocode if self.settings.analysis_backend().retains_prepared_session() => {
                return self.batch_pseudocode(address);
            }
            View::Pseudocode => return self.read_saved_pseudocode(address),
            View::Assembly => ("procedure_assembly", json!({"procedure":address})),
            View::Segments => ("list_segments", json!({})),
            _ => {
                return Err(crate::error::failure(
                    "This view does not have an engine reader",
                ));
            }
        };
        let reply = self.analysis_call(tool, args)?;
        bounded_view_text(reply)
    }

    fn read_saved_pseudocode(&mut self, address: &str) -> Result<String> {
        let result = self
            .engine()?
            .normalized("procedure_pseudo_code", json!({"procedure":address}))?;
        bounded_view_text(result)
    }

    fn read_dossier(&mut self, address: &str) -> Result<Value> {
        let project = self.project.as_ref().ok_or("Open a file first")?;
        if !self.settings.analysis_backend().retains_prepared_session()
            && let Some(text) =
                self.store
                    .cache(&project.target.sha256, address, "upstream:Dossier")?
        {
            return Ok(serde_json::from_str(&text)?);
        }
        let mut reply = self.analysis_call("analyze_function", json!({"procedure": address}))?;
        if let Some(value) = reply.pointer_mut("/raw_result/value") {
            return Ok(value.take());
        }
        match reply.get_mut("normalized_result") {
            Some(value) => Ok(value.take()),
            None => Ok(reply),
        }
    }
}

fn unselected_view(view: View) -> Result<String> {
    match view {
        View::References => Err(crate::error::failure(
            "Select a function to read its references",
        )),
        View::Graph => Ok(serde_json::to_string(
            &crate::analysis::graph::Graph::default(),
        )?),
        _ => Ok("Choose Analyze, then select a function to inspect it.".into()),
    }
}

fn bounded_view_text(mut reply: Value) -> Result<String> {
    let value = reply
        .get_mut("normalized_result")
        .map(Value::take)
        .unwrap_or(reply);
    let mut text = match value {
        Value::String(text) => text,
        value => bounded_json_text(&value)?,
    };
    if text.len() > files::TEXT_LIMIT {
        let mut end = files::TEXT_LIMIT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\nDisplay limited to 1 MiB.\n");
        // Provider-owned strings can have much larger backing allocations even
        // after truncation; do not carry those into retained UI code views.
        if text.capacity() > files::TEXT_LIMIT * 2 {
            text.shrink_to_fit();
        }
    }
    Ok(text)
}

fn bounded_json_text(value: &Value) -> Result<String> {
    let mut output = JsonPrefix {
        bytes: Vec::new(),
        overflowed: false,
    };
    let serialized = serde_json::to_writer_pretty(&mut output, value);
    if !output.overflowed {
        serialized?;
    }
    // A write can end within a UTF-8 character. Four extra bytes ensure that
    // removing its incomplete suffix still leaves > TEXT_LIMIT bytes whenever
    // the full serialization exceeded the limit. The caller adds the same
    // truncation notice and uses the same character boundary as before.
    if let Err(error) = std::str::from_utf8(&output.bytes) {
        if error.error_len().is_some() {
            return Err(error.into());
        }
        output.bytes.truncate(error.valid_up_to());
    }
    Ok(String::from_utf8(output.bytes)?)
}

struct JsonPrefix {
    bytes: Vec<u8>,
    overflowed: bool,
}

impl Write for JsonPrefix {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let remaining = (files::TEXT_LIMIT + 4).saturating_sub(self.bytes.len());
        if remaining == 0 && !bytes.is_empty() {
            self.overflowed = true;
            return Err(std::io::Error::other("Analysis preview is complete"));
        }
        let count = bytes.len().min(remaining);
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
