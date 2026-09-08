use crate::Value;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::collections::VecDeque;
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
    map: DashMap<String, MemoItem>,
    lists: DashMap<String, ReList>,
}

impl MemoryDB {
    pub fn new() -> Self {
        Self {
            map: DashMap::with_capacity(128),
            lists: DashMap::with_capacity(128),
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        let item = self.map.get(key)?;
        // println!("get map at : {:?}", Utc::now().timestamp_millis());
        if item.is_expired() {
            None
        } else {
            Some(item.value.clone())
        }
    }

    pub fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let item = self.map.get(key)?;
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

    fn get_or_create_list(&self, list_key: String) -> ReList {
        self.lists
            .entry(list_key)
            .or_insert_with(|| Arc::new(RwLock::new(VecDeque::with_capacity(64))))
            .value()
            .clone()
    }

    pub fn llen(&self, list_key: String) -> Value {
        let relist = self.lists.get(&list_key);
        let len = if let Some(relist) = relist {
            relist.read().unwrap().len() as i64
        } else {
            0i64
        };
        Value::Integer(len)
    }

    pub fn rpush(&self, list_key: String, val_iter: IntoIter<Value>) -> Value {
        let list_arc = self.get_or_create_list(list_key);
        let mut list = list_arc.write().unwrap();
        for value in val_iter {
            list.push_back(value);
        }
        Value::Integer(list.len() as i64)
    }

    pub fn lpush(&self, list_key: String, val_iter: IntoIter<Value>) -> Value {
        let list_arc = self.get_or_create_list(list_key);
        let mut list = list_arc.write().unwrap();
        for value in val_iter {
            list.push_front(value);
        }
        Value::Integer(list.len() as i64)
    }

    pub fn rpop(&self, list_key: String, cnt: i64) -> Value {
        self.pop_inner(list_key, cnt, false)
    }

    pub fn lpop(&self, list_key: String, cnt: i64) -> Value {
        self.pop_inner(list_key, cnt, true)
    }

    fn pop_inner(&self, list_key: String, mut cnt: i64, is_front: bool) -> Value {
        let relist = self.lists.get(&list_key);
        if let Some(list_rc) = relist {
            let mut list = list_rc.write().unwrap();
            cnt = cnt.min(list.len() as i64);
            let mut ans = Vec::new();
            for _ in 0..cnt {
                let val = if is_front {
                    list.pop_front().unwrap()
                } else {
                    list.pop_back().unwrap()
                };
                if cnt == 1 {
                    return val;
                }
                ans.push(val);
            }
            if ans.len() > 0 {
                return Value::Arrays(ans);
            }
        }
        return Value::NullBulkStrings;
    }

    pub fn lrange(&self, list_key: String, mut start: i64, mut end: i64) -> Value {
        let relist = self.lists.get(&list_key);
        let mut ans = Vec::new();
        if let Some(list_rc) = relist {
            let list = list_rc.read().unwrap();
            if start < 0 {
                start = 0.max(list.len() as i64 + start);
            }
            if end < 0 {
                end = 0.max(list.len() as i64 + end);
            }
            let start = start as usize;
            let mut end = end as usize;
            end = end.min(list.len() - 1);
            for i in start..=end {
                ans.push(list[i].clone());
            }
        }
        return Value::Arrays(ans);
    }
}
