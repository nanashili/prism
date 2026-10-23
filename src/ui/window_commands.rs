use super::app::Prism;
use crate::model::{Screen, View};

impl Prism {
    pub(super) fn receive_window_request(
        &mut self,
        context: &eframe::egui::Context,
        tool: &str,
        args: &serde_json::Value,
        answer: std::sync::mpsc::SyncSender<crate::error::Result<serde_json::Value>>,
        lifetime: crate::worker::RequestLifetime,
    ) {
        let result = match lifetime.check() {
            Err(error) => Err(error),
            Ok(()) if tool == "capture_window" => {
                self.capture_window(context, args, answer, lifetime);
                return;
            }
            Ok(()) => self.navigate(tool, args),
        };
        if answer.send(result).is_err() {
            self.status = "The agent closed its window request".into();
        }
    }

    pub(super) fn navigate(
        &mut self,
        tool: &str,
        args: &serde_json::Value,
    ) -> crate::error::Result<serde_json::Value> {
        use crate::error::failure;
        use serde_json::json;
        match tool {
            "open_file" => self.submit(crate::worker::Job::Open(absolute_path(args)?))?,
            "analyze" => self.submit(crate::worker::Job::Index)?,
            "save_project" => self.save_project_command(args)?,
            "get_state" => return Ok(self.window_state()),
            "current_view" => return Ok(self.current_view()),
            "list_marks" => return self.list_marks(),
            "ask_agent" => self.ask_agent_command(args)?,
            "open_function" => self.open_function_command(args)?,
            "show_screen" => self.show_screen(args)?,
            "set_view" => {
                let view = args["view"].as_str().ok_or("Unsupported view")?;
                self.select_view(parse_view(view)?);
            }
            "rename_function" | "add_note" | "bookmark" => {
                self.apply_annotation_tool(tool, args)?;
            }
            _ => return Err(failure(format!("Unsupported window command: {tool}"))),
        }
        Ok(json!({"updated":true}))
    }

    fn save_project_command(&mut self, args: &serde_json::Value) -> crate::error::Result<()> {
        let path = absolute_path(args)?;
        let project = self.workspace.project.clone().ok_or("Open a file first")?;
        self.submit(crate::worker::Job::Save {
            path,
            project,
            messages: self.chat.messages.clone(),
        })
    }

    fn ask_agent_command(&mut self, args: &serde_json::Value) -> crate::error::Result<()> {
        if self.chat.busy || self.pending > 0 {
            return Err("Wait for the current operation before asking the agent".into());
        }
        let prompt = args["prompt"].as_str().ok_or("Missing prompt")?;
        if prompt.trim().is_empty() || prompt.chars().count() > 32_000 {
            return Err("Prompts must contain 1 to 32,000 characters".into());
        }
        self.chat.draft = prompt.into();
        self.prepare_prompt()
    }

    fn open_function_command(&mut self, args: &serde_json::Value) -> crate::error::Result<()> {
        if self.workspace.project.is_none() {
            return Err("Open a file first".into());
        }
        let address = match args["function"].as_str() {
            Some(reference) => self.resolve_function(reference)?,
            None => args["address"]
                .as_str()
                .ok_or("Missing function address")?
                .into(),
        };
        let view = parse_view(args["view"].as_str().unwrap_or("Pseudocode"))?;

        self.screen = Screen::Workbench;
        if args["split"].as_bool().unwrap_or(false) {
            self.open_function_in_split(&address, &address, view);
        } else {
            self.open_function_view(&address, &address, view);
        }
        Ok(())
    }

    pub(super) fn resolve_function(&self, reference: &str) -> crate::error::Result<String> {
        let project = self.workspace.project.as_ref().ok_or("Open a file first")?;
        if project.renamed.contains_key(reference) {
            return Ok(reference.into());
        }
        let mut addresses: std::collections::BTreeSet<_> = self
            .store
            .find_function(&project.target.sha256, reference)?
            .into_iter()
            .collect();
        addresses.extend(
            project
                .renamed
                .iter()
                .filter(|(_, name)| name.as_str() == reference)
                .map(|(address, _)| address.clone()),
        );
        if addresses.len() > 1 {
            return Err("More than one function has that name. Use its address.".into());
        }
        if let Some(address) = addresses.into_iter().next() {
            return Ok(address);
        }
        if reference
            .strip_prefix("0x")
            .is_some_and(|address| u64::from_str_radix(address, 16).is_ok())
        {
            return Ok(reference.into());
        }
        Err("Function not found. Index the file first or use its address.".into())
    }
}

fn parse_view(label: &str) -> crate::error::Result<View> {
    View::ALL
        .into_iter()
        .find(|view| view.label().eq_ignore_ascii_case(label))
        .ok_or_else(|| crate::error::failure("Unsupported view"))
}

fn absolute_path(args: &serde_json::Value) -> crate::error::Result<std::path::PathBuf> {
    let path = args["path"]
        .as_str()
        .ok_or("An absolute path is required")?;
    let path = std::path::PathBuf::from(path);
    if !path.is_absolute() {
        return Err(crate::error::failure("An absolute path is required"));
    }
    Ok(path)
}
