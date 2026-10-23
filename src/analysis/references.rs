use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const LIMIT: usize = 1_000;

#[derive(Deserialize, Serialize)]
pub(crate) struct Reference {
    pub address: String,
    pub location: Option<String>,
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct References {
    pub incoming: Vec<Reference>,
    pub outgoing: Vec<Reference>,
    pub limited: bool,
}

impl References {
    pub fn from_dossier(value: &Value) -> Result<Self> {
        let callers = value["callers"]
            .as_array()
            .ok_or("The provider did not return callers")?;
        let callees = value["callees"]
            .as_array()
            .ok_or("The provider did not return callees")?;
        let sites = value["incoming_references"]
            .as_array()
            .filter(|sites| !sites.is_empty());
        let incoming_rows = sites.unwrap_or(callers);
        let incoming = incoming_rows
            .iter()
            .take(LIMIT)
            .map(incoming_reference)
            .collect::<Result<_>>()?;
        let outgoing = callees
            .iter()
            .take(LIMIT)
            .map(reference)
            .collect::<Result<_>>()?;

        Ok(Self {
            incoming,
            outgoing,
            limited: incoming_rows.len() > LIMIT
                || callees.len() > LIMIT
                || value["references_limited"].as_bool().unwrap_or(false),
        })
    }
}

fn incoming_reference(value: &Value) -> Result<Reference> {
    let Some(location) = value["source_address"].as_str() else {
        return reference(value);
    };
    let procedure = &value["source_procedure"];
    Ok(Reference {
        address: procedure["address"].as_str().unwrap_or(location).into(),
        location: Some(location.into()),
        name: procedure["name"].as_str().map(str::to_owned),
    })
}

fn reference(value: &Value) -> Result<Reference> {
    let address = value["address"]
        .as_str()
        .ok_or("A reference has no address")?;
    Ok(Reference {
        address: address.into(),
        location: None,
        name: value["name"].as_str().map(str::to_owned),
    })
}
