mod bsearch;
mod entry;

use crate::db::Signal;
use anyhow::{Ok, Result};
use bytes::Bytes;
use entry::EntryID;
use std::sync::{Arc, Mutex, RwLock};

pub use entry::StreamEntry;

#[derive(Debug)]
pub struct ReStream {
    _name: Bytes,
    stream: RwLock<Vec<StreamEntry>>,
    waiter_clients: Mutex<Vec<Arc<Signal>>>,
}

impl ReStream {
    pub fn register_waiter(&self, signal: Arc<Signal>) {
        let mut waiters = self.waiter_clients.lock().unwrap();
        if !waiters.iter().any(|w| Arc::ptr_eq(w, &signal)) {
            waiters.push(signal);
        }
    }

    pub fn unregister_waiter(&self, signal: Arc<Signal>) {
        let mut waiters = self.waiter_clients.lock().unwrap();
        waiters.retain(|w| !Arc::ptr_eq(w, &signal));
    }

    pub fn notify_all_waiters(&self) {
        let mut waiters = self.waiter_clients.lock().unwrap();
        for w in waiters.drain(..) {
            w.notify();
        }
    }
}

impl ReStream {
    pub fn new(name: Bytes) -> Self {
        Self {
            _name: name,
            stream: RwLock::new(Vec::new()),
            waiter_clients: Mutex::new(Vec::with_capacity(16)),
        }
    }

    pub fn len(&self) -> usize {
        self.stream.read().unwrap().len()
    }

    pub fn add(&self, id: Bytes, bytes_vec: Vec<Bytes>) -> Result<Bytes> {
        let cur_id = {
            let mut stream_guad = self.stream.write().unwrap();
            let last_id = stream_guad
                .last()
                .map(|e| e.id)
                .unwrap_or_else(|| EntryID::new(0, 0));
            let cur_id = last_id.next_id(id)?;
            stream_guad.push(StreamEntry::new(cur_id, bytes_vec));
            cur_id
        };
        self.notify_all_waiters();
        Ok(cur_id.to_bytes())
    }

    pub fn xrange(&self, start: Bytes, end: Bytes) -> Result<Vec<StreamEntry>> {
        let len = self.len();
        let start_idx = if &start[..] == b"-" {
            0
        } else {
            let start = EntryID::from(&start, 0)?;
            bsearch::lower_bound(&self.stream.read().unwrap(), start, true)
        };
        if start_idx == len {
            return Ok(Vec::new());
        }

        let end_idx = if &end[..] == b"+" {
            len - 1
        } else {
            let end = EntryID::from(&end, i64::MAX)?;
            bsearch::high_bound(&self.stream.read().unwrap(), end, true)
        };
        if end_idx == len || start_idx > end_idx {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(end_idx - start_idx + 1);
        let stream = self.stream.read().unwrap();
        for i in start_idx..=end_idx {
            results.push(stream[i].clone());
        }
        Ok(results)
    }

    pub fn xread(&self, id: &Bytes) -> Result<Vec<StreamEntry>> {
        let stream_guard = self.stream.read().unwrap();
        let n = stream_guard.len();
        let id = EntryID::from(id, 0)?;
        let idx = bsearch::lower_bound(&stream_guard, id, false);
        if idx == n {
            return Ok(Vec::new());
        }
        let mut results = Vec::with_capacity(n - idx);
        for i in idx..n {
            results.push(stream_guard[i].clone());
        }
        Ok(results)
    }

    pub fn last(&self) -> Option<StreamEntry> {
        let stream_guard = self.stream.read().unwrap();
        stream_guard.last().cloned()
    }
}
