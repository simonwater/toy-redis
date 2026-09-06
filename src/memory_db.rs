use std::collections::HashMap;

use crate::Value;

pub struct MemoryDB {
    map: HashMap<String, Value>,
}

impl MemoryDB {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_capacity(128),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.map.get(key)
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.map.insert(key, value);
    }
}
