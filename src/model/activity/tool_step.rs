use super::{Outcome, ToolStep, ToolUpdate, now_ms, prefix};

impl ToolStep {
    pub(in crate::model) fn start(id: String) -> Self {
        Self {
            id,
            title: String::new(),
            status: Outcome::Running,
            started: now_ms(),
            ended: None,
            input: String::new(),
            output: String::new(),
        }
    }

    pub(in crate::model) fn apply(&mut self, update: ToolUpdate) -> bool {
        let mut limited = false;
        for (value, destination, limit) in [
            (update.title, &mut self.title, 1024),
            (update.input, &mut self.input, 16 * 1024),
            (update.output, &mut self.output, 16 * 1024),
        ] {
            if let Some(value) = value {
                limited |= value.len() > limit;
                *destination = prefix(&value, limit).into();
            } else if destination.len() > limit {
                *destination = prefix(destination, limit).into();
                limited = true;
            }
        }
        if let Some(status) = update.status {
            self.status = status;
            self.ended = (status != Outcome::Running).then(now_ms);
        }
        limited
    }

    pub(in crate::model) fn retained_bytes(&self) -> usize {
        self.id.len() + self.title.len() + self.input.len() + self.output.len()
    }
}
