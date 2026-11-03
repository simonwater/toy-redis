use std::{
    collections::HashSet,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};

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

    pub fn subscirbe(&mut self, name: Bytes, ctx: &Arc<Context>) -> Option<Receiver<Bytes>> {
        if self.chennals.insert(name.clone()) {
            let rx = ctx.chennals_hub().subscribe(name);
            return Some(rx);
        }
        None
    }

    pub fn len(&self) -> usize {
        self.chennals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chennals.is_empty()
    }
}

pub struct ChennalsHub {
    chennals: DashMap<Bytes, Vec<Sender<Bytes>>>,
}

impl ChennalsHub {
    pub fn new() -> Self {
        Self {
            chennals: DashMap::with_capacity(16),
        }
    }

    pub fn subscribe(&self, chennal: Bytes) -> Receiver<Bytes> {
        let (tx, rx) = mpsc::channel();

        let mut subs = self
            .chennals
            .entry(chennal)
            .or_insert_with(|| Vec::with_capacity(16));
        subs.push(tx);
        rx
    }

    pub fn publish(&self, chennal: Bytes, msg: Bytes) -> usize {
        if let Some(mut e) = self.chennals.get_mut(&chennal) {
            let subs = e.value_mut();
            subs.retain(|tx| tx.send(msg.clone()).is_ok());
            return subs.len();
        }
        0
    }

    pub fn subscriber_len(&self, chennal: Bytes) -> usize {
        match self.chennals.get(&chennal) {
            Some(v) => v.value().len(),
            _ => 0,
        }
    }
}
