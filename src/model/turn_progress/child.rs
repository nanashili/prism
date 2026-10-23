use super::super::activity::append_recent;
use super::super::agent_updates::ChildEvent;
use super::{Child, ToolStep};

impl Child {
    pub(super) fn apply(&mut self, event: ChildEvent) -> bool {
        let limited = match event {
            ChildEvent::Text(text) => append_recent(&mut self.text, &text, 16 * 1024),
            ChildEvent::Thought(text) => append_recent(&mut self.thought, &text, 4096),
            ChildEvent::Tool(update) => {
                if let Some(tool) = self.tools.iter_mut().find(|tool| tool.id == update.id) {
                    tool.apply(update)
                } else {
                    let full = self.tools.len() >= 32;
                    if full {
                        self.tools.remove(0);
                    }
                    let mut tool = ToolStep::start(update.id.clone());
                    let limited = tool.apply(update);
                    self.tools.push(tool);
                    limited || full
                }
            }
        };
        self.retain_recent() || limited
    }

    fn retain_recent(&mut self) -> bool {
        let mut bytes = self.retained_bytes();
        let mut dropped = 0;
        for tool in &self.tools {
            if bytes <= 64 * 1024 && self.tools.len() - dropped <= 32 {
                break;
            }
            bytes -= tool.retained_bytes();
            dropped += 1;
        }
        self.tools.drain(..dropped);
        dropped > 0
    }

    pub(super) fn retained_bytes(&self) -> usize {
        self.id.len()
            + self.parent.as_ref().map_or(0, String::len)
            + self.name.len()
            + self.task.len()
            + self.status.len()
            + self.text.len()
            + self.thought.len()
            + self
                .tools
                .iter()
                .map(ToolStep::retained_bytes)
                .sum::<usize>()
    }
}
