use crate::Value;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::vec::IntoIter;

pub const DAY_IN_MILLIS: i64 = 1000 * 60 * 60 * 24;
type ReList = Arc<RwLock<VecDeque<Value>>>;

#[derive(Debug, Clone)]
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
    lists: DashMap<String, ReList>,
}

impl MemoryDB {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_capacity(128),
            lists: DashMap::with_capacity(128),
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

    fn get_or_create_list(&self, list_key: String) -> ReList {
        self.lists
            .entry(list_key)
            .or_insert_with(|| Arc::new(RwLock::new(VecDeque::with_capacity(64))))
            .value()
            .clone()
    }

    pub fn rpush(&self, list_key: String, val_iter: IntoIter<Value>) -> Value {
        let list_arc = self.get_or_create_list(list_key);
        let mut list = list_arc.write().unwrap();
        for value in val_iter {
            list.push_back(value);
        }
        Value::Integer(list.len() as i64)
    }
}
