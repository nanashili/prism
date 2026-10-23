use crate::error::Result;
use crate::model::View;

#[derive(Default)]
pub(super) struct ViewTab {
    pub identity: u64,
    pub preview: bool,
    pub tab_closed: bool,
    pub jump: String,
    pub history: Vec<(String, View)>,
    pub history_at: usize,
    pub selected: String,
    pub view: View,
    pub text: String,
    pub lines: Vec<std::ops::Range<usize>>,
    pub hex_offset: u64,
    pub rename: String,
    pub patch: String,
    pub note: String,
    pub graph: Option<super::graph::GraphView>,
    pub references: Option<crate::analysis::references::References>,
    pub request: u64,
    pub retain_output: bool,
}

impl ViewTab {
    pub fn duplicate_selection(&self) -> Self {
        Self {
            selected: self.selected.clone(),
            view: self.view,
            hex_offset: self.hex_offset,
            rename: self.rename.clone(),
            jump: self.jump.clone(),
            ..Default::default()
        }
    }

    pub fn release_content(&mut self) {
        // Commands with side effects cannot be replayed when a tab becomes visible again.
        if self.retain_output {
            return;
        }
        self.request = 0;
        self.text = String::new();
        self.lines = Vec::new();
        self.graph = None;
        self.references = None;
    }

    pub fn receive_content(&mut self, view: View, address: &str, text: String) -> Result<()> {
        if self.view != view || (!address.is_empty() && self.selected != address) {
            return Ok(());
        }
        if view == View::Graph {
            let graph = serde_json::from_str(&text)
                .map_err(|error| format!("Invalid control-flow graph: {error}"))?;
            self.graph = Some(super::graph::GraphView::new(graph));
        }
        if view == View::References {
            self.references = Some(
                serde_json::from_str(&text)
                    .map_err(|error| format!("Invalid references: {error}"))?,
            );
        }
        self.set_text(text);
        Ok(())
    }

    pub fn set_text(&mut self, text: String) {
        let mut start = 0;
        self.lines = text
            .split_inclusive('\n')
            .map(|line| {
                let range = start..start + line.len();
                start += line.len();
                range
            })
            .collect();
        self.text = text;
    }
}
