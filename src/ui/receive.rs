use super::app::{Permission, Prism};
use super::notifications::Level;
use crate::events::Event;
use crate::model::Screen;

impl Prism {
    pub(super) fn receive(&mut self, context: &eframe::egui::Context) {
        self.capture_error();
        for _ in 0..64 {
            let event = self
                .events
                .as_ref()
                .and_then(|events| events.try_recv().ok());
            let Some(event) = event else {
                break;
            };
            match event {
                Event::Chat { key, event } => self.receive_chat_event(context, key, *event),
                event => self.receive_event(context, event),
            }
            self.capture_error();
        }
    }

    pub(super) fn receive_event(&mut self, context: &eframe::egui::Context, event: Event) {
        match event {
            Event::Chat { .. } => unreachable!("chat events are unwrapped before dispatch"),
            Event::Progress(text) => {
                if let Some(progress) = &mut self.analysis
                    && progress.total.is_none()
                {
                    progress.label = text.clone();
                }
                self.status = text;
            }
            Event::Analysis(progress) => self.analysis = progress,
            Event::ExecutionEvidenceChanged(project) => {
                if let Err(error) = self.refresh_execution_evidence(&project) {
                    self.report_error(error);
                }
            }
            Event::Reconstruction {
                project,
                status,
                start_agent,
            } => {
                self.receive_reconstruction(&project, *status, start_agent);
            }
            Event::Opened(project, document) => {
                self.debug.isolated = false;
                self.reconstruction = Default::default();
                self.reconstruction_root = None;
                self.restore_document_chat(&project, document);

                self.workspace = super::app::Workspace {
                    project: Some(*project),
                    ..Default::default()
                };
                self.restore_sidebar_preferences();
                if self.settings.conversation.notify_when_ready {
                    self.notify(
                        Level::Success,
                        "File ready",
                        "Your file is ready to explore.",
                        "",
                    );
                }
                self.screen = if self.settings.conversation.open_workbench == "Only when I ask"
                    || (self.chat.busy && self.settings.conversation.open_workbench != "Always")
                {
                    Screen::Home
                } else {
                    Screen::Workbench
                };
                if let Err(error) = self.refresh_chats() {
                    self.report_error(error);
                }
                match self.store.recent(self.settings.recent_limit) {
                    Ok(recent) => self.recent = recent,
                    Err(error) => self.report_error(error),
                }
            }
            Event::Functions { rows, offset } => {
                self.workspace.rows = rows;
                self.workspace.offset = offset;
                self.workspace.navigation.invalidate();
            }
            Event::Sidebar {
                project,
                generation,
                parent,
                offset,
                page,
            } => {
                self.receive_sidebar(&project, generation, parent, offset, page);
            }
            Event::Resources {
                project,
                generation,
                parent,
                offset,
                page,
            } => {
                self.receive_resources(&project, generation, parent, offset, page);
            }
            Event::Clipboard(text) => context.copy_text(text),
            Event::Content {
                request,
                view,
                address,
                text,
            } => {
                let active = self.workspace.active;
                let pane = match request {
                    Some(request) => self
                        .workspace
                        .panes
                        .iter_mut()
                        .flat_map(|pane| pane.tabs.iter_mut())
                        .find(|tab| request == tab.request && !tab.tab_closed),
                    None => {
                        let tab = &mut self.workspace.tabs[active];
                        tab.view = view;
                        tab.tab_closed = false;
                        Some(tab)
                    }
                };
                if let Some(pane) = pane
                    && let Err(error) = pane.receive_content(view, &address, text)
                {
                    self.report_error(error);
                }
            }
            Event::Exported(path) => {
                self.notify(
                    Level::Success,
                    "Export saved",
                    &path.display().to_string(),
                    "",
                );
                if self.settings.desktop.reveal_exports
                    && let Err(error) = super::settings_files::reveal(&path)
                {
                    self.notify(
                        Level::Warning,
                        "Couldn't show the export",
                        "Your export was saved. Finder could not open its location.",
                        &error.to_string(),
                    );
                }
            }
            Event::Saved(path) => {
                if let Some(project) = &mut self.workspace.project {
                    project.source_document = Some(path.clone());
                }
                self.notify(
                    Level::Success,
                    "Project saved",
                    &path.display().to_string(),
                    "",
                );
                self.workspace.document = Some(path);
            }
            Event::Finished(result) => {
                self.pending = self.pending.saturating_sub(1);
                match result {
                    Ok(text) if !text.is_empty() => self.status = text,
                    Err(error) if error.to_string() == "Cancelled" => {
                        self.status = "Stopped".into();
                    }
                    Err(error) => {
                        self.status.clear();
                        self.report_error(error);
                    }
                    Ok(_) if self.pending == 0 => self.status = "Ready".into(),
                    _ => {}
                }
            }
            Event::AgentText(text) => self.append_agent_text(text),
            Event::AgentThought(text) => {
                if let Some(activity) = self
                    .chat
                    .messages
                    .last_mut()
                    .and_then(|m| m.activity.as_mut())
                {
                    activity.record_thought(&text);
                }
            }
            Event::AgentTool(update) => {
                if let Some(activity) = self
                    .chat
                    .messages
                    .last_mut()
                    .and_then(|m| m.activity.as_mut())
                {
                    activity.update_tool(update);
                }
            }
            Event::AgentUpdate(update) => self.receive_agent_update(update),
            Event::AgentReady(options) => self.receive_agent_options(options),
            Event::Permission {
                request,
                answer,
                active,
            } => {
                if !active.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                if self.settings.conversation.notify_when_asked {
                    self.notify_permission(&request.title);
                }
                self.chat.permission = Some(Permission {
                    request,
                    answer,
                    active,
                })
            }
            Event::Question {
                question,
                answer,
                active,
            } => {
                self.receive_question(question, answer, active);
            }
            Event::AgentFinished { result, stopped } => {
                if (result.is_ok() || stopped) && self.settings.conversation.notify_when_done {
                    self.notify_agent_complete(stopped);
                }
                self.finish_agent(result, stopped);
            }
            Event::Navigate {
                tool,
                args,
                answer,
                lifetime,
            } => self.receive_window_request(context, &tool, &args, answer, lifetime),
        }
    }
}
