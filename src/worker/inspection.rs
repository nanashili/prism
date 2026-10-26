use super::State;
use crate::{error::Result, model::View};
use serde_json::{Value, json};

impl State {
    pub(super) fn inspect_selection(&mut self, args: &Value) -> Result<Value> {
        let project = self.project.as_ref().ok_or("Open a file first")?;
        if args["project"].as_str() != Some(&project.target.sha256) {
            return Err("The open project changed before the selection was read".into());
        }
        let address = args["address"]
            .as_str()
            .ok_or("A selection address is required")?;
        let target = project.target.clone();
        let mut result = json!({"project":target.sha256,"address":address});

        match crate::analysis::graph::address_value(address) {
            Some(address) => match crate::analysis::file_inventory::selection(&target, address) {
                Ok(selection) => result["selection"] = selection,
                Err(error) => result["selection_error"] = error.to_string().into(),
            },
            None => {
                result["selection_error"] = "This selection has no native memory address".into();
            }
        }
        match self.read_view(View::References, address, 0) {
            Ok(text) => result["references"] = serde_json::from_str(&text)?,
            Err(error) => result["references_error"] = error.to_string().into(),
        }
        Ok(result)
    }
}
