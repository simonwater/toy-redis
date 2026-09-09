use crate::Value;
use crate::list::ReList;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use std::vec::IntoIter;

pub const DAY_IN_MILLIS: i64 = 1000 * 60 * 60 * 24;

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
    map: DashMap<String, MemoItem>,
    lists: DashMap<String, Arc<ReList>>,
}

impl MemoryDB {
    pub fn new() -> Self {
        Self {
            map: DashMap::with_capacity(128),
            lists: DashMap::with_capacity(128),
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        let item = self.map.get(key).map(|v| v.value().clone())?;
        // println!("get map at : {:?}", Utc::now().timestamp_millis());
        if item.is_expired() {
            None
        } else {
            Some(item.value.clone())
        }
    }

    pub fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let item = self.map.get(key).map(|v| v.value().clone())?;
        if item.is_expired() {
            None
        } else {
            let mut bytes = Vec::with_capacity(512);
            item.value.serilize(&mut bytes);
            Some(bytes)
        }
    }

    pub fn set(&self, key: String, value: Value) {
        self.set_with_ttl(key, value, DAY_IN_MILLIS);
    }

    pub fn set_with_ttl(&self, key: String, value: Value, ttl_ms: i64) {
        let item = MemoItem::new(value, ttl_ms);
        // println!(
        //     "set map item: {:?}, at: {}",
        //     item,
        //     Utc::now().timestamp_millis()
        // );
        self.map.insert(key, item);
    }

    /// lists

    fn get_or_create_list(&self, list_key: String) -> Arc<ReList> {
        self.lists
            .entry(list_key.clone())
            .or_insert_with(|| Arc::new(ReList::new(list_key)))
            .value()
            .clone()
    }

    pub fn rpush(&self, list_key: String, val_iter: IntoIter<Value>) -> Value {
        let list_arc = self.get_or_create_list(list_key);
        list_arc.push_back(val_iter);
        list_arc.len()
    }

    pub fn lpush(&self, list_key: String, val_iter: IntoIter<Value>) -> Value {
        let list_arc = self.get_or_create_list(list_key);
        list_arc.push_front(val_iter);
        list_arc.len()
    }

    pub fn rpop(&self, list_key: String, cnt: i64) -> Value {
        let relist = self.lists.get(&list_key).map(|v| v.value().clone());
        if let Some(list_arc) = relist {
            return list_arc.pop_back(cnt);
        }
        return Value::NullBulkStrings;
    }

    pub fn lpop(&self, list_key: String, cnt: i64) -> Value {
        let relist = self.lists.get(&list_key).map(|v| v.value().clone());
        if let Some(list_arc) = relist {
            return list_arc.pop_front(cnt);
        }
        return Value::NullBulkStrings;
    }

    pub fn brpop(&self, list_key: String, timeout_in_sec: f64) -> Value {
        let list_arc = self.get_or_create_list(list_key);
        return list_arc.block_pop_back(timeout_in_sec);
    }

    pub fn blpop(&self, list_key: String, timeout_in_sec: f64) -> Value {
        let list_arc = self.get_or_create_list(list_key);
        return list_arc.block_pop_front(timeout_in_sec);
    }

    pub fn lrange(&self, list_key: String, start: i64, end: i64) -> Value {
        let relist = self.lists.get(&list_key).map(|v| v.value().clone());
        if let Some(list_arc) = relist {
            return list_arc.range(start, end);
        }
        return Value::Arrays(Vec::new());
    }

    pub fn llen(&self, list_key: String) -> Value {
        let relist = self.lists.get(&list_key).map(|v| v.value().clone());
        if let Some(list_arc) = relist {
            return list_arc.len();
        }
        return Value::Integer(0);
    }
}
