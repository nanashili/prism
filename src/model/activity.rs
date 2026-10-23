mod retention;
mod tool_step;

pub(super) use retention::append_recent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    #[default]
    Running,
    Done,
    Failed,
    Stopped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Activity {
    pub agent: String,
    pub started: i64,
    pub ended: Option<i64>,
    pub outcome: Outcome,
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub limited: bool,
    #[serde(default)]
    pub dropped_blocks: usize,
    #[serde(default)]
    pub dropped_thoughts: usize,
    #[serde(default)]
    pub dropped_text_end: usize,
    #[serde(default)]
    pub progress: super::TurnProgress,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Block {
    Text { start: usize, end: usize },
    Thought { text: String },
    Tool(ToolStep),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ToolStep {
    pub id: String,
    pub title: String,
    pub status: Outcome,
    pub started: i64,
    pub ended: Option<i64>,
    pub input: String,
    pub output: String,
}

pub(crate) struct ToolUpdate {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<Outcome>,
    pub input: Option<String>,
    pub output: Option<String>,
}

impl Activity {
    pub fn start(agent: &str) -> Self {
        Self {
            agent: agent.into(),
            started: now_ms(),
            ended: None,
            outcome: Outcome::Running,
            blocks: Vec::new(),
            limited: false,
            dropped_blocks: 0,
            dropped_thoughts: 0,
            dropped_text_end: 0,
            progress: Default::default(),
        }
    }

    pub fn record_text(&mut self, start: usize, end: usize) {
        if let Some(Block::Text { end: previous, .. }) = self.blocks.last_mut() {
            *previous = end;
        } else {
            self.push_block(Block::Text { start, end });
        }
        self.retain_recent();
    }

    pub fn record_thought(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(Block::Thought { text: previous }) = self.blocks.last_mut() {
            self.limited |= append_recent(previous, text, 16 * 1024);
        } else {
            let mut preview = String::new();
            self.limited |= append_recent(&mut preview, text, 16 * 1024);
            self.push_block(Block::Thought { text: preview });
        }
        self.retain_recent();
    }

    pub fn update_tool(&mut self, update: ToolUpdate) {
        if let Some(Block::Tool(tool)) = self
            .blocks
            .iter_mut()
            .find(|block| matches!(block, Block::Tool(tool) if tool.id == update.id))
        {
            self.limited |= tool.apply(update);
        } else {
            let mut tool = ToolStep::start(update.id.clone());
            self.limited |= tool.apply(update);
            self.push_block(Block::Tool(tool));
        }
        self.retain_recent();
    }

    pub fn finish(&mut self, outcome: Outcome) {
        self.progress.finish();
        self.retain_recent();
        self.outcome = outcome;
        self.ended = Some(now_ms());
        for block in &mut self.blocks {
            if let Block::Tool(tool) = block
                && tool.status == Outcome::Running
            {
                tool.status = Outcome::Stopped;
                tool.ended = self.ended;
            }
        }
    }

    pub fn retained_bytes(&self) -> usize {
        self.progress.retained_bytes()
            + self.blocks.iter().map(Block::retained_bytes).sum::<usize>()
    }
}

pub(crate) fn prefix(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}
