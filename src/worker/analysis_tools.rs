use super::State;
use crate::error::{Result, failure};
use serde_json::{Value, json};

const MAX_SELECTED_TOOLS: usize = 32;

struct DiscoveryRequest {
    full: bool,
    names: Option<Vec<String>>,
}

impl State {
    pub(super) fn analysis_tools(&mut self, args: &Value) -> Result<Value> {
        let request = DiscoveryRequest::parse(args)?;
        let provider = self.engine()?.tools()?;
        project_provider_tools(&provider, &request)
    }
}

impl DiscoveryRequest {
    fn parse(args: &Value) -> Result<Self> {
        let full = match args.get("detail") {
            None => false,
            Some(value) => match value.as_str() {
                Some("summary") => false,
                Some("full") => true,
                Some(_) => return Err(failure("Tool discovery detail must be summary or full")),
                None => return Err(failure("Tool discovery detail must be a string")),
            },
        };

        Ok(Self {
            full,
            names: selected_names(args)?,
        })
    }
}

fn project_provider_tools(provider: &Value, request: &DiscoveryRequest) -> Result<Value> {
    let source = provider["tools"]
        .as_array()
        .ok_or("The provider returned no tool catalog")?;
    let allowed = source
        .iter()
        .filter(|tool| {
            tool["name"]
                .as_str()
                .is_some_and(|name| !blocked_tool(name) && !requires_batch(name))
        })
        .collect();

    let selected = select_tools(allowed, request.names.as_deref())?;
    if request.full {
        let tools: Vec<_> = selected.into_iter().cloned().collect();
        let mut result = provider.clone();
        result["tools"] = tools.into();
        return Ok(result);
    }

    let tools: Vec<_> = selected.into_iter().map(summary_tool).collect();
    Ok(json!({"tools":tools}))
}

fn select_tools<'a>(allowed: Vec<&'a Value>, names: Option<&[String]>) -> Result<Vec<&'a Value>> {
    if let Some(names) = names {
        for name in names {
            if !allowed
                .iter()
                .any(|tool| tool["name"].as_str() == Some(name))
            {
                return Err(failure(format!("Unknown analysis tool: {name}")));
            }
        }
    }

    Ok(allowed
        .into_iter()
        .filter(|tool| {
            names.is_none_or(|names| {
                tool["name"]
                    .as_str()
                    .is_some_and(|name| names.iter().any(|selected| selected == name))
            })
        })
        .collect())
}

fn summary_tool(tool: &Value) -> Value {
    json!({
        "name": tool["name"],
        "description": tool["description"],
        "required": tool["inputSchema"]["required"].as_array().cloned().unwrap_or_default(),
    })
}

fn selected_names(args: &Value) -> Result<Option<Vec<String>>> {
    let Some(raw) = args.get("names") else {
        return Ok(None);
    };
    let values = raw.as_array().ok_or("Tool names must be an array")?;
    if values.is_empty() || values.len() > MAX_SELECTED_TOOLS {
        return Err(failure(format!(
            "Select 1–{MAX_SELECTED_TOOLS} analysis tools"
        )));
    }
    values
        .iter()
        .map(|value| {
            let name = value.as_str().ok_or("Tool names must be strings")?;
            if name.is_empty() || name.chars().count() > 256 {
                return Err(failure("Analysis tool names must be 1–256 characters"));
            }
            Ok(name.to_owned())
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

pub(super) fn requires_batch(name: &str) -> bool {
    matches!(name, "procedure_pseudo_code" | "batch_decompile")
}

pub(super) fn blocked_tool(name: &str) -> bool {
    matches!(
        name,
        "open_binary"
            | "close_binary"
            | "set_address_name"
            | "set_addresses_names"
            | "set_comment"
            | "set_inline_comment"
            | "set_bookmark"
            | "unset_bookmark"
            | "set_code_type"
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{DiscoveryRequest, MAX_SELECTED_TOOLS, project_provider_tools};

    #[test]
    fn summary_is_bounded_and_full_named_selection_preserves_schema() {
        let provider = json!({
            "tools": [
                {
                    "name": "allowed",
                    "description": "Useful guidance",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"value": {"type": "string"}},
                        "required": ["value"]
                    },
                    "outputSchema": {"type": "object"}
                },
                {
                    "name": "set_comment",
                    "description": "Do not expose",
                    "inputSchema": {"type": "object"}
                },
                {
                    "name": "procedure_pseudo_code",
                    "description": "Use Prism's durable batch instead",
                    "inputSchema": {"type": "object"}
                }
            ],
            "metadata": {"provider": "fixture"}
        });
        let summary_request = DiscoveryRequest::parse(&json!({"detail":"summary"}))
            .expect("Summary discovery is a valid request");
        let summary = project_provider_tools(&provider, &summary_request)
            .expect("The fixture has a provider catalog");
        assert_eq!(
            summary["tools"]
                .as_array()
                .expect("Summary tools are an array")
                .len(),
            1
        );
        assert_eq!(summary["tools"][0]["description"], "Useful guidance");
        assert_eq!(summary["tools"][0]["required"], json!(["value"]));
        assert!(summary["tools"][0].get("inputSchema").is_none());
        assert!(summary["tools"][0].get("outputSchema").is_none());

        let unfiltered_request = DiscoveryRequest::parse(&json!({"detail":"full"}))
            .expect("Full discovery is a valid request");
        let unfiltered_catalog = project_provider_tools(&provider, &unfiltered_request)
            .expect("The fixture has a provider catalog");
        assert_eq!(
            unfiltered_catalog["tools"]
                .as_array()
                .expect("Full tools are an array")
                .len(),
            1
        );
        assert_eq!(unfiltered_catalog["metadata"], provider["metadata"]);

        let full_request = DiscoveryRequest::parse(&json!({"detail":"full","names":["allowed"]}))
            .expect("Named full discovery is a valid request");
        let full = project_provider_tools(&provider, &full_request)
            .expect("The selected fixture tool is available");
        assert_eq!(full["tools"][0], provider["tools"][0]);
    }

    #[test]
    fn unknown_and_blocked_names_are_rejected() {
        let provider = json!({"tools":[{"name":"set_comment","description":"blocked"}]});
        let unknown = DiscoveryRequest::parse(&json!({"names":["missing"]}))
            .expect("An unknown name still has a valid request shape");
        let blocked = DiscoveryRequest::parse(&json!({"names":["set_comment"]}))
            .expect("A blocked name still has a valid request shape");
        assert!(project_provider_tools(&provider, &unknown).is_err());
        assert!(project_provider_tools(&provider, &blocked).is_err());
    }

    #[test]
    fn discovery_request_rejects_invalid_detail_and_excessive_selection() {
        assert!(DiscoveryRequest::parse(&json!({"detail":true})).is_err());
        assert!(DiscoveryRequest::parse(&json!({
            "names": (0..=MAX_SELECTED_TOOLS).map(|index| format!("tool-{index}")).collect::<Vec<_>>()
        }))
        .is_err());
    }
}
