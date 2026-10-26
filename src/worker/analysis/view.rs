use super::State;
use crate::error::Result;
use crate::model::View;
use crate::project::files;
use serde_json::{Value, json};

impl State {
    pub(in crate::worker) fn engine_view(&mut self, view: View, address: &str) -> Result<String> {
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
            #[cfg(feature = "rea-rust-ghidra")]
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
        value => serde_json::to_string_pretty(&value)?,
    };
    if text.len() > files::TEXT_LIMIT {
        let mut end = files::TEXT_LIMIT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\nDisplay limited to 1 MiB.\n");
    }
    Ok(text)
}
