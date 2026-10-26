use super::State;
use crate::error::{Result, failure};
use serde_json::{Value, json};

impl State {
    pub(super) fn remote(&mut self, tool: &str, args: Value) -> Result<Value> {
        match tool {
            "compare_versions" => self.compare_versions(&args),
            "version_matches" => self.version_matches(&args),
            "investigation" => {
                let project = &self
                    .project
                    .as_ref()
                    .ok_or("Open a file first")?
                    .target
                    .sha256;
                let result = crate::investigation::call(&self.store, project, &args)?;
                if !matches!(args["action"].as_str(), Some("list" | "status"))
                    && let Some(id) = result["graph"]["id"].as_str()
                {
                    self.sink.send(crate::events::Event::InvestigationChanged {
                        project: project.clone(),
                        id: id.into(),
                    })?;
                }
                Ok(result)
            }
            "execution_evidence" => self.execution_evidence(&args),
            "recover_types" => {
                let project = self.project.as_ref().ok_or("Open a file first")?;
                if args["project"].as_str() != Some(project.target.sha256.as_str()) {
                    return Err(failure("The open project changed before type recovery"));
                }
                if !matches!(project.target.kind.as_str(), "app" | "binary") {
                    return Err(failure("Type recovery currently requires a native binary"));
                }
                let context = self.decompilation_context()?;
                let analyzer = super::analysis_cache_identity::namespace(&context);
                let mut payload = args;
                payload
                    .as_object_mut()
                    .ok_or("Supply type recovery arguments")?
                    .remove("project");
                self.store
                    .recover_types(&context.project, &analyzer, &payload)
            }
            name if name.starts_with("reconstruction_") => self.reconstruct(name, &args),
            "decompile_batch" => self.decompile_batch(&args),
            "analysis_repair" => self.analysis_repair(&args),
            "inspect_selection" => self.inspect_selection(&args),
            "index_function_search" => self.index_function_search(&args),
            "semantic_search" => self.semantic_search(&args),
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
