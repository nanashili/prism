#[derive(Default)]
pub(in crate::ui) struct MessageIdentities {
    next: u64,
    ids: Vec<u64>,
}

impl MessageIdentities {
    pub fn ensure(&mut self, count: usize) {
        while self.ids.len() < count {
            self.ids.push(self.next);
            self.next += 1;
        }
    }

    pub fn get(&self, index: usize) -> u64 {
        self.ids[index]
    }

    pub fn remove_front(&mut self, count: usize) {
        self.ids.drain(..count.min(self.ids.len()));
    }
}
