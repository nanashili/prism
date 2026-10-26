use super::app::Prism;
use crate::model::View;
use crate::ui::presentation::HistoryDirection;
use crate::worker::{Job, ViewJob};

impl Prism {
    pub(super) fn view_relevant(&self, view: View) -> bool {
        match view {
            View::Debug => self.settings.research.debugger(),
            View::Reconstruct => self.settings.research.native_tools(),
            _ => true,
        }
    }

    pub(super) fn open_hex_at(&mut self, offset: u64) {
        self.focus_current_pane();
        if self.prepare_tool_tab(View::Hex) {
            self.workspace.hex_offset = offset;
            self.select_view(View::Hex);
        }
    }

    pub(super) fn select_view(&mut self, view: View) {
        self.focus_current_pane();
        if !self.prepare_tool_tab(view) {
            return;
        }
        self.workspace.tab_closed = false;
        let destination = (self.workspace.selected.clone(), view);
        if self.workspace.history.get(self.workspace.history_at) != Some(&destination) {
            let retained = self.workspace.history_at + 1;
            self.workspace.history.truncate(retained);
            self.workspace.history.push(destination);
            if self.workspace.history.len() > 100 {
                self.workspace.history.remove(0);
            }
            self.workspace.history_at = self.workspace.history.len() - 1;
        }
        self.load_view(view);
    }

    pub(super) fn navigate_history(&mut self, forward: HistoryDirection) {
        let forward = forward == HistoryDirection::Forward;

        let index = if forward {
            self.workspace.history_at + 1
        } else {
            self.workspace.history_at.saturating_sub(1)
        };
        if let Some((address, view)) = self.workspace.history.get(index).cloned() {
            self.workspace.history_at = index;
            self.workspace.selected = address;
            self.workspace.tab_closed = false;
            self.load_view(view);
        }
    }

    pub(super) fn load_view(&mut self, view: View) {
        self.workspace.view = view;
        self.workspace.retain_output = false;
        self.workspace.graph = None;
        self.workspace.references = None;
        self.workspace.set_text(String::new());
        if view == View::Notes {
            self.next_view_request();
            self.workspace.note = self
                .workspace
                .project
                .as_ref()
                .and_then(|project| project.notes.get(&self.workspace.selected))
                .cloned()
                .unwrap_or_default();
        } else {
            self.enqueue_view(ViewJob::Content {
                view,
                address: self.workspace.selected.clone(),
                offset: self.workspace.hex_offset,
            });
        }
    }

    pub(super) fn enqueue_view(&mut self, job: ViewJob) {
        self.focus_current_pane();
        self.workspace.retain_output = !matches!(job, ViewJob::Content { .. });
        let request = self.next_view_request();
        self.enqueue(Job::View { request, job });
    }

    fn next_view_request(&mut self) -> u64 {
        self.workspace.next_request = self.workspace.next_request.wrapping_add(1).max(1);
        let request = self.workspace.next_request;
        self.workspace.request = request;
        request
    }
}
