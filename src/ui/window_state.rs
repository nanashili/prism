use super::app::Prism;

impl Prism {
    pub(super) fn current_view(&self) -> serde_json::Value {
        serde_json::json!({
            "file":self.workspace.project.as_ref().map(|project| &project.target.path),
            "function":self.workspace.project.as_ref().and_then(|project| project.renamed.get(&self.workspace.selected))
                .unwrap_or(&self.workspace.selected),
            "address":self.workspace.selected,"view":self.workspace.view.label().to_lowercase()
        })
    }

    pub(super) fn list_marks(&self) -> crate::error::Result<serde_json::Value> {
        let project = self.workspace.project.as_ref().ok_or("Open a file first")?;
        Ok(serde_json::json!({
            "renamed":project.renamed,"notes":project.notes,"bookmarks":project.bookmarks
        }))
    }

    pub(super) fn window_state(&self) -> serde_json::Value {
        use serde_json::json;
        json!({
            "screen":format!("{:?}", self.screen),
            "target":self.workspace.project.as_ref().map(|project| &project.target),
            "name":self.workspace.project.as_ref().and_then(|project| project.renamed.get(&self.workspace.selected)),
            "note":self.workspace.project.as_ref().and_then(|project| project.notes.get(&self.workspace.selected)),
            "function":self.workspace.selected,"view":self.workspace.view.label(),
            "panes":self.workspace.panes.iter()
                .take(if self.workspace.split { 2 } else { 1 })
                .enumerate()
                .map(|(index, pane)| json!({
                    "function":pane.selected,"view":pane.view.label(),
                    "focused":index == self.workspace.focused_pane,
                    "closed":pane.tab_closed,"active_tab":pane.active,
                    "tabs":pane.tabs.iter().filter(|tab| !tab.tab_closed)
                        .map(|tab| json!({"function":tab.selected,"view":tab.view.label(),"preview":tab.preview}))
                        .collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            "visible_text":self.workspace.text.chars().take(32_000).collect::<String>(),
            "pending":self.pending,"status":self.status,
            "analysis":self.analysis,
            "error":self.error.as_deref().or_else(|| self.notifications.active_error()),
            "notifications":{"unread":self.notifications.unread(),"open":self.notifications.open},
            "functions":self.workspace.rows.len(),"document":self.workspace.document,
            "chat":{
                "key":self.chat_tabs.active,
                "question_pending":self.chat.questions.is_pending(),
                "busy":self.chat.busy,"status":self.chat.status,
                "reply":self.chat.messages.last()
                    .filter(|message| message.role == "assistant")
                    .map(|message| message.text.chars().take(16_000).collect::<String>())
            }
        })
    }
}
