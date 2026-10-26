use super::app::Prism;
use crate::model::{Screen, View};
use crate::ui::presentation::{HistoryDirection, OpenTarget, SaveDestination};

impl Prism {
    pub(super) fn run_shortcut(&mut self, id: &str) {
        match id {
            "open" => self.choose_open(OpenTarget::File),
            "save" => self.save_document(SaveDestination::Current),
            "thread" => self.new_chat(),
            "palette" => self.palette = !self.palette,
            "settings" => self.screen = Screen::Settings,
            "stop" => {
                self.chat.questions.clear_agent_question();
                if let Some(agent) = &self.agent {
                    agent.cancel();
                }
            }
            _ if self.screen == Screen::Workbench && self.workspace.project.is_some() => {
                self.workbench_shortcut(id);
            }
            _ => {}
        }
    }

    fn workbench_shortcut(&mut self, id: &str) {
        match id {
            "undo" => self.undo_edit(),
            "back" => self.navigate_history(HistoryDirection::Backward),
            "forward" => self.navigate_history(HistoryDirection::Forward),
            "rename" | "comment" => self.select_view(View::Notes),
            "xrefs" => self.select_view(View::References),
            "graph" => self.select_view(View::Graph),
            "pseudo" => self.select_view(View::Pseudocode),
            "strings" => self.select_view(View::Strings),
            "patch" => self.select_view(View::Hex),
            "debug" => self.select_view(View::Debug),
            "split" => self.toggle_workbench_split(),
            "agent" => self.workspace.agent_closed = !self.workspace.agent_closed,
            "bookmark" => {
                let address = self.workspace.selected.clone();
                if address.is_empty() {
                    return;
                }
                self.edit_project(|project| {
                    if project.bookmarks.contains(&address) {
                        project.bookmarks.retain(|entry| entry != &address);
                    } else {
                        project.bookmarks.push(address);
                    }
                });
            }
            _ => {}
        }
    }
}
