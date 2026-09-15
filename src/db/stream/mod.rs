mod bsearch;
mod entry;

use anyhow::{Ok, Result};
use bytes::Bytes;
use entry::EntryID;
use std::sync::RwLock;

pub use entry::StreamEntry;

#[derive(Debug)]
pub struct ReStream {
    _name: Bytes,
    stream: RwLock<Vec<StreamEntry>>,
}

impl ReStream {
    pub fn new(name: Bytes) -> Self {
        Self {
            _name: name,
            stream: RwLock::new(Vec::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.stream.read().unwrap().len()
    }

    pub fn add(&self, id: Bytes, bytes_vec: Vec<Bytes>) -> Result<Bytes> {
        let mut s = self.stream.write().unwrap();
        let last_id = s.last().map(|e| e.id).unwrap_or_else(|| EntryID::new(0, 0));
        let cur_id = last_id.next_id(id)?;
        s.push(StreamEntry::new(cur_id, bytes_vec));
        Ok(cur_id.to_bytes())
    }

    pub fn xrange(&self, start: Bytes, end: Bytes) -> Result<Vec<StreamEntry>> {
        let len = self.len();
        let start_idx = if &start[..] == b"-" {
            0
        } else {
            let start = EntryID::from(&start, 0)?;
            bsearch::lower_bound(self, start, true)
        };
        if start_idx == len {
            return Ok(Vec::new());
        }

        let end_idx = if &end[..] == b"+" {
            len - 1
        } else {
            let end = EntryID::from(&end, i64::MAX)?;
            bsearch::high_bound(self, end, true)
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
        let n = self.len();
        let id = EntryID::from(id, 0)?;
        let idx = bsearch::lower_bound(self, id, false);
        if idx == n {
            return Ok(Vec::new());
        }
        let mut results = Vec::with_capacity(n - idx);
        let stream = self.stream.read().unwrap();
        for i in idx..n {
            results.push(stream[i].clone());
        }
        Ok(results)
    }
}
