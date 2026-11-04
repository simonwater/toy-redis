use std::{
    collections::HashMap,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};

use bytes::Bytes;
use dashmap::DashMap;

use crate::Context;

pub struct ChenalMessage {
    pub(crate) chennal: Bytes,
    pub(crate) message: Bytes,
}

impl ChenalMessage {
    pub(crate) fn new(chennal: Bytes, message: Bytes) -> Self {
        Self { chennal, message }
    }
}

/// 客户端的订阅
pub struct Subscription {
    chenal_listeners: HashMap<Bytes, Receiver<Bytes>>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
            chenal_listeners: HashMap::with_capacity(8),
        }
    }

    pub fn subscirbe(&mut self, name: Bytes, ctx: &Arc<Context>) -> usize {
        // 确保不会重复订阅
        self.chenal_listeners
            .entry(name.clone())
            .or_insert_with(|| ctx.chennals_hub().subscribe(name));

        self.chenal_listeners.len()
    }

    pub fn unsubscribe(&mut self, name: Bytes, _ctx: &Arc<Context>) -> usize {
        self.chenal_listeners.remove(&name);
        self.chenal_listeners.len()
    }

    pub fn try_recv_msgs(&self) -> Vec<ChenalMessage> {
        let mut result = Vec::with_capacity(self.chenal_listeners.len());
        for (chennal, rx) in &self.chenal_listeners {
            if let Ok(msg) = rx.try_recv() {
                result.push(ChenalMessage::new(chennal.clone(), msg));
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.chenal_listeners.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chenal_listeners.is_empty()
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
