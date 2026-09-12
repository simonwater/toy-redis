use std::collections::BTreeMap;
use std::sync::RwLock;

use bytes::Bytes;

#[derive(Debug)]
pub struct ReStream {
    _name: Bytes,
    stream: RwLock<BTreeMap<Bytes, Vec<Bytes>>>,
}

impl ReStream {
    pub fn new(name: Bytes) -> Self {
        Self {
            _name: name,
            stream: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn add(&self, key: Bytes, bytes_vec: Vec<Bytes>) -> Bytes {
        self.stream.write().unwrap().insert(key.clone(), bytes_vec);
        key
    }
}
