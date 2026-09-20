mod list;
mod redis_object;
mod signal;
mod stream;

use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use redis_object::RedisObject;
use std::sync::Arc;

pub use list::ReList;
pub(crate) use signal::Signal;
pub use stream::{ReStream, StreamEntry};

pub const DAY_IN_MILLIS: i64 = 1000 * 60 * 60 * 24;

#[derive(Clone, Copy, Debug)]
struct ExpirationTimestamp(i64);

impl ExpirationTimestamp {
    fn from_ttl(mut ttl_ms: i64) -> Self {
        if ttl_ms <= 0 {
            ttl_ms = DAY_IN_MILLIS;
        }
        let expire_at = Utc::now() + Duration::milliseconds(ttl_ms);
        let expire_timestamp_ms = expire_at.timestamp_millis();
        Self(expire_timestamp_ms)
    }

    fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() >= self.0
    }
}

#[derive(Debug)]
struct MemoItem {
    object: RedisObject,
    expire_timestamp_ms: ExpirationTimestamp, // 毫秒表示的过期时间戳
    version: u64,
}

impl MemoItem {
    fn new(object: RedisObject) -> Self {
        Self::with_ttl(object, 0)
    }

    fn with_ttl(object: RedisObject, ttl_ms: i64) -> Self {
        Self {
            object,
            expire_timestamp_ms: ExpirationTimestamp::from_ttl(ttl_ms),
            version: 0,
        }
    }

    fn is_expired(&self) -> bool {
        self.expire_timestamp_ms.is_expired()
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

    pub fn get_version(&self, key: &Bytes) -> Option<u64> {
        if let Some(entry) = self.map.get(key)
            && !entry.is_expired()
        {
            return Some(entry.version);
        }
        None
    }

    pub fn set(&self, key: Bytes, bytes: Bytes) {
        self.set_with_ttl(key, bytes, DAY_IN_MILLIS);
    }

    pub fn incr(&self, key: Bytes) -> Result<i64> {
        match self.map.entry(key) {
            dashmap::Entry::Occupied(mut entry) => {
                let item = entry.get_mut();
                match &mut item.object {
                    RedisObject::String(val_bytes) => {
                        let val_str = std::str::from_utf8(&val_bytes)
                            .map_err(|_| anyhow!("ERR value is not an integer or out of range"))?;
                        let mut val: i64 = val_str
                            .parse()
                            .map_err(|_| anyhow!("ERR value is not an integer or out of range"))?;
                        val = val
                            .checked_add(1)
                            .ok_or_else(|| anyhow!("ERR increment or decrement would overflow"))?;

                        *val_bytes = Bytes::from(val.to_string());
                        Ok(val)
                    }
                    _ => bail!("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            dashmap::Entry::Vacant(entry) => {
                entry.insert(MemoItem::new(RedisObject::new_integer(1)));
                Ok(1)
            }
        }
    }

    pub fn set_with_ttl(&self, key: Bytes, bytes: Bytes, ttl_ms: i64) {
        let mut e = self
            .map
            .entry(key)
            .or_insert_with(|| MemoItem::with_ttl(RedisObject::String(bytes.clone()), ttl_ms));
        let item = e.value_mut();
        item.version += 1;
        item.object = RedisObject::String(bytes);
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

    fn get_list(&self, list_key: &Bytes) -> Result<Option<Arc<ReList>>> {
        self.map
            .get(list_key)
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
        if let Some(list_arc) = self.get_list(&list_key)? {
            return Ok(list_arc.pop_back(cnt));
        }
        return Ok(None);
    }

    pub fn lpop(&self, list_key: Bytes, cnt: i64) -> Result<Option<Vec<Bytes>>> {
        if let Some(list_arc) = self.get_list(&list_key)? {
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
        if let Some(list_arc) = self.get_list(&list_key)? {
            return Ok(Some(list_arc.range(start, end)));
        }
        Ok(None)
    }

    pub fn llen(&self, list_key: Bytes) -> Result<i64> {
        if let Some(list_arc) = self.get_list(&list_key)? {
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

    fn get_stream(&self, stream_key: &Bytes) -> Result<Option<Arc<ReStream>>> {
        self.map
            .get(stream_key)
            .map(|entry| entry.value().object.to_restream())
            .transpose()
    }

    pub fn xadd(&self, stream_key: Bytes, entry_id: Bytes, val_vec: Vec<Bytes>) -> Result<Bytes> {
        let stream_arc = self.get_or_create_stream(stream_key)?;
        let id = stream_arc.add(entry_id, val_vec)?;
        Ok(id)
    }

    pub fn xrange(
        &self,
        stream_key: Bytes,
        start: Bytes,
        end: Bytes,
    ) -> Result<Option<Vec<StreamEntry>>> {
        if let Some(stream_arc) = self.get_stream(&stream_key)? {
            return Ok(Some(stream_arc.xrange(start, end)?));
        }
        Ok(None)
    }

    pub fn xread(
        &self,
        stream_keys: &[Bytes],
        entry_ids: &[Bytes],
    ) -> Result<Option<Vec<(Bytes, Vec<StreamEntry>)>>> {
        if stream_keys.is_empty() || stream_keys.len() != entry_ids.len() {
            bail!("ERR stream keys ans entry ids format error");
        }
        let mut results = Vec::with_capacity(stream_keys.len());
        for (stream_key, id) in stream_keys.iter().zip(entry_ids) {
            let stream_arc = self.get_or_create_stream(stream_key.clone())?;

            let entrys = stream_arc.xread(id)?;

            results.push((stream_key.clone(), entrys));
        }

        Ok(Some(results))
    }

    pub fn block_xread(
        &self,
        stream_keys: Vec<Bytes>,
        mut entry_ids: Vec<Bytes>,
        mut timeout_in_milli: i64,
    ) -> Result<Option<Vec<(Bytes, Vec<StreamEntry>)>>> {
        if stream_keys.is_empty() || stream_keys.len() != entry_ids.len() {
            bail!("ERR stream keys ans entry ids format error");
        }
        if timeout_in_milli <= 0 || timeout_in_milli > 3600_000 {
            timeout_in_milli = 3600_000;
        }
        let total_dur = Duration::milliseconds(timeout_in_milli);
        let expr_at = Utc::now() + total_dur; // 过期时间

        self.handle_stream_id(&stream_keys, &mut entry_ids)?;
        let mut results = Vec::with_capacity(stream_keys.len());
        if self.try_xread_without_empty(&stream_keys, &entry_ids, &mut results)? {
            return Ok(Some(results));
        }
        let signal = Arc::new(Signal::new());
        self.register_signal(&stream_keys, &signal)?;
        loop {
            let is_notified = signal.wait_until(expr_at);
            // 超时边界的数据都满足要求
            if self.try_xread_without_empty(&stream_keys, &entry_ids, &mut results)? {
                return Ok(Some(results));
            }
            if !is_notified {
                // 超时
                return Ok(None);
            }
        }
    }

    fn handle_stream_id(&self, stream_keys: &[Bytes], entry_ids: &mut [Bytes]) -> Result<()> {
        for i in 0..stream_keys.len() {
            let stream_arc = self.get_or_create_stream(stream_keys[i].clone())?;
            if "$" == std::str::from_utf8(&entry_ids[i])? {
                let id_bytes = if let Some(entry) = stream_arc.last() {
                    entry.get_id().to_bytes()
                } else {
                    Bytes::from("0-0")
                };
                entry_ids[i] = id_bytes;
            }
        }
        Ok(())
    }

    fn register_signal(&self, stream_keys: &[Bytes], signal: &Arc<Signal>) -> Result<()> {
        for stream_key in stream_keys {
            let stream_arc = self.get_or_create_stream(stream_key.clone())?;
            stream_arc.register_waiter(Arc::clone(signal));
        }
        Ok(())
    }

    fn try_xread_without_empty(
        &self,
        stream_keys: &[Bytes],
        entry_ids: &[Bytes],
        results: &mut Vec<(Bytes, Vec<StreamEntry>)>,
    ) -> Result<bool> {
        for (stream_key, id) in stream_keys.iter().zip(entry_ids) {
            let stream_arc = self.get_or_create_stream(stream_key.clone())?;
            let entrys = stream_arc.xread(id)?;
            if !entrys.is_empty() {
                results.push((stream_key.clone(), entrys));
            }
        }
        Ok(!results.is_empty())
    }
}
