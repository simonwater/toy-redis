use std::collections::BTreeMap;
use std::sync::RwLock;

use bytes::Bytes;

#[derive(Debug)]
pub struct ReStream {
    stream: RwLock<BTreeMap<Bytes, Vec<Bytes>>>,
}

impl ReStream {
    pub fn new() -> Self {
        Self {
            stream: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn add(&self, key: Bytes, bytes_vec: Vec<Bytes>) -> Bytes {
        self.stream.write().unwrap().insert(key.clone(), bytes_vec);
        key
    }
}
