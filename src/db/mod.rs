mod list;
mod redis_object;
mod stream;

use anyhow::Result;
use bytes::Bytes;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use list::ReList;
use redis_object::RedisObject;
use std::sync::Arc;
use stream::ReStream;

pub const DAY_IN_MILLIS: i64 = 1000 * 60 * 60 * 24;

#[derive(Debug)]
struct MemoItem {
    object: RedisObject,
    expire_timestamp_ms: i64, // 毫秒表示的过期时间戳
}

impl MemoItem {
    fn new(object: RedisObject) -> Self {
        Self::with_ttl(object, 0)
    }

    fn with_ttl(object: RedisObject, mut ttl_ms: i64) -> Self {
        if ttl_ms <= 0 {
            ttl_ms = DAY_IN_MILLIS;
        }
        let expire_at = Utc::now() + Duration::milliseconds(ttl_ms);
        let expire_timestamp_ms = expire_at.timestamp_millis();
        Self {
            object,
            expire_timestamp_ms,
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() >= self.expire_timestamp_ms
    }
}

pub struct MemoryDB {
    map: DashMap<Bytes, MemoItem>,
}

impl MemoryDB {
    pub fn new() -> Self {
        Self {
            map: DashMap::with_capacity(128),
        }
    }

    pub fn obj_type(&self, key: &Bytes) -> String {
        if let Some(item) = self.map.get(key)
            && !item.is_expired()
        {
            return item.object.obj_type();
        }
        return "none".into();
    }

    pub fn get(&self, key: &Bytes) -> Result<Option<Bytes>> {
        if let Some(item) = self.map.get(key)
            && !item.is_expired()
        {
            let bytes = item.object.to_bulk_bytes()?;
            return Ok(Some(bytes));
        }
        return Ok(None);
    }

    pub fn set(&self, key: Bytes, bytes: Bytes) {
        self.set_with_ttl(key, bytes, DAY_IN_MILLIS);
    }

    pub fn set_with_ttl(&self, key: Bytes, bytes: Bytes, ttl_ms: i64) {
        let item = MemoItem::with_ttl(RedisObject::String(bytes), ttl_ms);
        self.map.insert(key, item);
    }
}

/// lists
impl MemoryDB {
    fn get_or_create_list(&self, list_key: Bytes) -> Result<Arc<ReList>> {
        self.map
            .entry(list_key.clone())
            .or_insert_with(|| MemoItem::new(RedisObject::new_list(list_key)))
            .value()
            .object
            .to_relist()
    }

    fn get_list(&self, list_key: Bytes) -> Result<Option<Arc<ReList>>> {
        self.map
            .get(&list_key)
            .map(|entry| entry.value().object.to_relist())
            .transpose()
    }

    pub fn rpush(&self, list_key: Bytes, val_iter: Vec<Bytes>) -> Result<i64> {
        let list_arc = self.get_or_create_list(list_key)?;
        let len = list_arc.push_back(val_iter);
        Ok(len)
    }

    pub fn lpush(&self, list_key: Bytes, val_iter: Vec<Bytes>) -> Result<i64> {
        let list_arc = self.get_or_create_list(list_key)?;
        let len = list_arc.push_front(val_iter);
        Ok(len)
    }

    pub fn rpop(&self, list_key: Bytes, cnt: i64) -> Result<Option<Vec<Bytes>>> {
        if let Some(list_arc) = self.get_list(list_key)? {
            return Ok(list_arc.pop_back(cnt));
        }
        return Ok(None);
    }

    pub fn lpop(&self, list_key: Bytes, cnt: i64) -> Result<Option<Vec<Bytes>>> {
        if let Some(list_arc) = self.get_list(list_key)? {
            return Ok(list_arc.pop_front(cnt));
        }
        return Ok(None);
    }

    pub fn brpop(&self, list_key: Bytes, timeout_in_sec: f64) -> Result<Option<Bytes>> {
        let list_arc = self.get_or_create_list(list_key)?;
        return Ok(list_arc.block_pop_back(timeout_in_sec));
    }

    pub fn blpop(&self, list_key: Bytes, timeout_in_sec: f64) -> Result<Option<Bytes>> {
        let list_arc = self.get_or_create_list(list_key)?;
        return Ok(list_arc.block_pop_front(timeout_in_sec));
    }

    pub fn lrange(&self, list_key: Bytes, start: i64, end: i64) -> Result<Option<Vec<Bytes>>> {
        if let Some(list_arc) = self.get_list(list_key)? {
            return Ok(Some(list_arc.range(start, end)));
        }
        Ok(None)
    }

    pub fn llen(&self, list_key: Bytes) -> Result<i64> {
        if let Some(list_arc) = self.get_list(list_key)? {
            return Ok(list_arc.len());
        }
        Ok(0)
    }
}

/// stream
impl MemoryDB {
    fn get_or_create_stream(&self, stream_key: Bytes) -> Result<Arc<ReStream>> {
        self.map
            .entry(stream_key.clone())
            .or_insert_with(|| MemoItem::new(RedisObject::new_stream(stream_key)))
            .value()
            .object
            .to_restream()
    }

    pub fn xadd(&self, steam_key: Bytes, entry_id: Bytes, val_vec: Vec<Bytes>) -> Result<Bytes> {
        let stream_arc = self.get_or_create_stream(steam_key)?;
        let id = stream_arc.add(entry_id, val_vec);
        Ok(id)
    }
}
