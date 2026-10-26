use super::State;
use crate::error::{Result, failure};
use serde_json::{Value, json};

impl State {
    pub(super) fn remote(&mut self, tool: &str, args: Value) -> Result<Value> {
        match tool {
            "execution_evidence" => self.execution_evidence(&args),
            name if name.starts_with("reconstruction_") => self.reconstruct(name, &args),
            "decompile_batch" => self.decompile_batch(&args),
            "inspect_selection" => self.inspect_selection(&args),
            "list_functions" => {
                let project = self.project.as_ref().ok_or("Open a file first")?;
                let offset = usize::try_from(args["offset"].as_u64().unwrap_or_default())?;
                let rows = self.store.functions(
                    &project.target.sha256,
                    args["filter"].as_str().unwrap_or_default(),
                    offset,
                )?;
                Ok(json!({"functions":rows,"offset":offset,"page_size":300}))
            }
            "read_bytes" => {
                let project = self.project.as_ref().ok_or("Open a file first")?;
                let offset = args["offset"]
                    .as_u64()
                    .ok_or("A nonnegative file offset is required")?;
                let length = args["length"].as_u64().unwrap_or(256).min(4096) as usize;
                let bytes = crate::project::files::bytes(&project.target.path, offset, length)?;
                let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                Ok(json!({"offset":offset,"length":bytes.len(),"bytes_hex":hex}))
            }
            "analysis_tools" => {
                let mut result = self.engine()?.tools()?;
                if let Some(tools) = result["tools"].as_array_mut() {
                    tools.retain(|tool| {
                        let name = tool["name"].as_str().unwrap_or_default();
                        !blocked_tool(name) && !requires_batch(name)
                    });
                }
                Ok(result)
            }
            "analysis_call" => {
                let name = args["name"]
                    .as_str()
                    .ok_or("An analysis tool name is required")?;
                if blocked_tool(name) {
                    return Err(failure(
                        "Use Prism's file and annotation tools to preserve the open project and undo history",
                    ));
                }
                if requires_batch(name) {
                    return Err(failure(
                        "Use decompile_batch for native decompilation, or reconstruction_evidence for a claimed reconstruction batch",
                    ));
                }
                self.analysis_call(name, args.get("arguments").cloned().unwrap_or(json!({})))
            }
            _ => Err(failure(format!("Unknown Prism tool: {tool}"))),
        }
    }
}

fn requires_batch(name: &str) -> bool {
    matches!(name, "procedure_pseudo_code" | "batch_decompile")
}

fn blocked_tool(name: &str) -> bool {
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
