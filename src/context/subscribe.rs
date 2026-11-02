use std::collections::HashSet;

use bytes::Bytes;

pub struct Subscription {
    chennals: HashSet<Bytes>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
            chennals: HashSet::with_capacity(8),
        }
    }

    pub fn add_chennal(&mut self, name: Bytes) {
        self.chennals.insert(name);
    }

    pub fn len(&self) -> usize {
        self.chennals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chennals.is_empty()
    }
}
