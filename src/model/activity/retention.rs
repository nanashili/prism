use super::{Activity, Block};

const MAX_BLOCKS: usize = 2048;
const MAX_ACTIVITY_BYTES: usize = 256 * 1024;

impl Activity {
    pub(super) fn push_block(&mut self, block: Block) {
        if self.blocks.len() >= MAX_BLOCKS {
            self.drop_oldest(self.blocks.len() - MAX_BLOCKS + 32);
        }
        self.blocks.push(block);
    }

    pub(crate) fn retain_recent(&mut self) {
        self.progress.retain_recent();
        let budget = MAX_ACTIVITY_BYTES.saturating_sub(self.progress.retained_bytes());
        let mut bytes: usize = self.blocks.iter().map(Block::retained_bytes).sum();
        let mut dropped = 0;

        for block in &self.blocks {
            if bytes <= budget && self.blocks.len() - dropped <= MAX_BLOCKS {
                break;
            }
            bytes -= block.retained_bytes();
            dropped += 1;
        }

        self.drop_oldest(dropped);
    }

    fn drop_oldest(&mut self, dropped: usize) {
        if dropped > 0 {
            for block in self.blocks.drain(..dropped) {
                match block {
                    Block::Thought { .. } => {
                        self.dropped_thoughts = self.dropped_thoughts.saturating_add(1);
                    }
                    Block::Text { end, .. } => {
                        self.dropped_text_end = self.dropped_text_end.max(end);
                    }
                    Block::Tool(_) => {}
                }
            }
            self.dropped_blocks = self.dropped_blocks.saturating_add(dropped);
            self.limited = true;
        }
    }
}

impl Block {
    pub(super) fn retained_bytes(&self) -> usize {
        match self {
            Self::Text { .. } => 0,
            Self::Thought { text } => text.len(),
            Self::Tool(tool) => tool.retained_bytes(),
        }
    }
}

pub(in crate::model) fn append_recent(destination: &mut String, text: &str, limit: usize) -> bool {
    let size = destination.len().saturating_add(text.len());
    if size <= limit {
        destination.push_str(text);
        return false;
    }

    if text.len() >= limit {
        *destination = suffix(text, limit).into();
    } else {
        let retained = suffix(destination, limit - text.len()).len();
        destination.drain(..destination.len() - retained);
        destination.push_str(text);
    }
    true
}

fn suffix(text: &str, limit: usize) -> &str {
    let mut start = text.len().saturating_sub(limit);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}
