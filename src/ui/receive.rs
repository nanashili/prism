mod workspace;

use super::app::{Permission, Prism};
use crate::error::Result;
use crate::events::Event;
use crate::model::Approval;
use crate::ui::presentation::{AgentLaunch, ChatOrigin, TurnEnd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::SyncSender};

impl Prism {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
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
                    progress.label.clone_from(&text);
                }
                self.status = text;
            }
            Event::Analysis(progress) => self.analysis = progress,
            Event::InvestigationCreated(graph) => self.receive_investigation(*graph),
            Event::ExecutionEvidenceChanged(project) => {
                if let Err(error) = self.refresh_execution_evidence(&project) {
                    self.report_error(error);
                }
            }
            Event::InvestigationChanged { project, id } => {
                if self
                    .workspace
                    .project
                    .as_ref()
                    .is_some_and(|opened| opened.target.sha256 == project)
                    && let Err(error) = self.refresh_investigation(Some(&id))
                {
                    self.report_error(error);
                }
            }
            Event::Reconstruction {
                project,
                status,
                start_agent,
            } => {
                self.receive_reconstruction(&project, *status, AgentLaunch::from(start_agent));
            }
            Event::Comparison(report) => {
                if self
                    .workspace
                    .project
                    .as_ref()
                    .is_some_and(|project| project.target.sha256 == report.target_sha256)
                {
                    self.workspace.comparison = Some(report);
                    self.workspace.reviewed_matches.clear();
                }
            }
            Event::Opened(project, document) => {
                self.receive_opened(context, *project, ChatOrigin::from(document));
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
            } => self.receive_content(request, view, &address, text),
            Event::Exported(path) => self.receive_exported(&path),
            Event::Saved(path) => self.receive_saved(path),
            Event::Finished(result) => self.finish_operation(result),
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
            } => self.receive_permission(request, answer, active),
            Event::Question {
                question,
                answer,
                active,
            } => {
                self.receive_question(question, answer, active);
            }
            Event::AgentFinished { result, stopped } => {
                if (result.is_ok() || stopped) && self.settings.conversation.notify_when_done {
                    self.notify_agent_complete(TurnEnd::from(stopped));
                }
                self.finish_agent(result, TurnEnd::from(stopped));
            }
            Event::Navigate {
                tool,
                args,
                answer,
                lifetime,
            } => self.receive_window_request(context, &tool, &args, answer, lifetime),
        }
    }

    fn finish_operation(&mut self, result: Result<String>) {
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

    fn receive_permission(
        &mut self,
        request: Approval,
        answer: SyncSender<Option<String>>,
        active: Arc<AtomicBool>,
    ) {
        if !active.load(Ordering::Relaxed) {
            return;
        }
        if self.settings.conversation.notify_when_asked {
            self.notify_permission(&request.title);
        }
        self.chat.permission = Some(Permission {
            request,
            answer,
            active,
        });
    }
}
