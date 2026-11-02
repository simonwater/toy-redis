use std::{collections::HashSet, sync::Arc};

use bytes::Bytes;
use dashmap::DashMap;

use crate::Context;

/// 客户端的订阅
pub struct Subscription {
    chennals: HashSet<Bytes>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
            chennals: HashSet::with_capacity(8),
        }
    }

    pub fn add_chennal(&mut self, name: Bytes, ctx: &Arc<Context>) {
        if self.chennals.insert(name.clone()) {
            ctx.subcriber_hub().add_client(name);
        }
    }

    pub fn len(&self) -> usize {
        self.chennals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chennals.is_empty()
    }
}

pub struct SubscriberHub {
    subscribers: DashMap<Bytes, usize>,
}

impl SubscriberHub {
    pub fn new() -> Self {
        Self {
            subscribers: DashMap::with_capacity(16),
        }
    }

    pub fn add_client(&self, chennal: Bytes) {
        let mut v = self.subscribers.entry(chennal).or_insert(0);
        *v += 1;
    }

    pub fn client_cnt(&self, chennal: Bytes) -> usize {
        match self.subscribers.get(&chennal) {
            Some(v) => *v.value(),
            _ => 0,
        }
    }
}
