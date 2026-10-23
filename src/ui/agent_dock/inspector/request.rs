use crate::{ui::app::Prism, worker::Job};
use eframe::egui;
use std::sync::mpsc;

impl Prism {
    pub(super) fn prepare_inspection(&mut self) -> bool {
        let project = self
            .workspace
            .project
            .as_ref()
            .map(|project| project.target.sha256.clone());
        if self.chat_tabs.inspection.project == project
            && self.chat_tabs.inspection.address == self.workspace.selected
        {
            if self.chat_tabs.inspection.save_at.is_none() {
                self.chat_tabs.inspection.note = self
                    .workspace
                    .project
                    .as_ref()
                    .and_then(|project| project.notes.get(&self.workspace.selected))
                    .cloned()
                    .unwrap_or_default();
            }
            return true;
        }
        if let Err(error) = self.save_inspector_note() {
            self.report_error(error);
            return false;
        }
        self.chat_tabs.inspection = super::Inspection {
            project,
            address: self.workspace.selected.clone(),
            note: self
                .workspace
                .project
                .as_ref()
                .and_then(|project| project.notes.get(&self.workspace.selected))
                .cloned()
                .unwrap_or_default(),
            ..Default::default()
        };
        self.request_inspection();
        true
    }

    pub(super) fn request_inspection(&mut self) {
        let (answer, pending) = mpsc::sync_channel(1);
        let job = Job::Remote {
            tool: "inspect_selection".into(),
            args: serde_json::json!({"project":self.chat_tabs.inspection.project,"address":self.chat_tabs.inspection.address}),
            answer,
            lifetime: None,
        };
        let result = self
            .worker
            .as_ref()
            .ok_or("The analysis worker is closed")
            .map_err(crate::error::failure)
            .and_then(|worker| worker.submit(job));
        let inspection = &mut self.chat_tabs.inspection;
        inspection.data = serde_json::Value::Null;
        inspection.error = None;
        match result {
            Ok(()) => inspection.pending = Some(pending),
            Err(error) => inspection.error = Some(error.to_string()),
        }
    }

    pub(in crate::ui) fn update_inspector(&mut self, context: &egui::Context) {
        let inspection = &mut self.chat_tabs.inspection;
        if let Some(pending) = &inspection.pending {
            match pending.try_recv() {
                Ok(result) => {
                    inspection.pending = None;
                    match result {
                        Ok(data) => inspection.data = data,
                        Err(error) => inspection.error = Some(error.to_string()),
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    context.request_repaint_after(std::time::Duration::from_millis(80))
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    inspection.pending = None;
                    inspection.error = Some("The analysis worker closed before replying".into());
                }
            }
        }
        if let Some(due) = inspection.save_at {
            if due <= std::time::Instant::now() {
                if let Err(error) = self.save_inspector_note() {
                    self.report_error(error);
                    self.chat_tabs.inspection.save_at =
                        Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
                }
            } else {
                context.request_repaint_after(
                    due.saturating_duration_since(std::time::Instant::now()),
                );
            }
        }
    }
}
