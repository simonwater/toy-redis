use crate::Value;
use chrono::{Duration, Utc};
use std::collections::HashMap;

pub const DAY_IN_MILLIS: i64 = 1000 * 60 * 60 * 24;

#[derive(Debug)]
struct MemoItem {
    value: Value,
    expire_timestamp_ms: i64, // 毫秒表示的过期时间戳
}

impl MemoItem {
    fn new(value: Value, ttl_ms: i64) -> Self {
        let expire_at = Utc::now() + Duration::milliseconds(ttl_ms);
        let expire_timestamp_ms = expire_at.timestamp_millis();
        Self {
            value,
            expire_timestamp_ms,
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() >= self.expire_timestamp_ms
    }
}

pub struct MemoryDB {
    map: HashMap<String, MemoItem>,
}

impl MemoryDB {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_capacity(128),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        let item = self.map.get(key)?;
        // println!("get map at : {:?}", Utc::now().timestamp_millis());
        if item.is_expired() {
            None
        } else {
            Some(&item.value)
        }
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.set_with_ttl(key, value, DAY_IN_MILLIS);
    }

    pub fn set_with_ttl(&mut self, key: String, value: Value, ttl_ms: i64) {
        let item = MemoItem::new(value, ttl_ms);
        // println!(
        //     "set map item: {:?}, at: {}",
        //     item,
        //     Utc::now().timestamp_millis()
        // );
        self.map.insert(key, item);
    }
}
