mod bitmap;
mod geo;
mod list;
mod redis_object;
mod signal;
mod sorted_set;
mod stream;

use crate::db::sorted_set::ZSetEntry;
use crate::{RdbItem, RdbParser, RdbValue, RedisArgs};
use anyhow::{Context, Ok, Result, anyhow, bail};
use bytes::{Bytes, BytesMut};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use list::ReList;
use redis_object::RedisObject;
use signal::Signal;
use sorted_set::SortedSet;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) use crate::db::geo::Coordinates;
pub(crate) use stream::{ReStream, StreamEntry};

pub const DEFAULT_TTL_MILLIS: i64 = 1000 * 60 * 60 * 24; // 一天

#[derive(Clone, Copy, Debug)]
struct ExpirationTimestamp(i64);

impl ExpirationTimestamp {
    fn from_ttl(mut ttl_ms: i64) -> Self {
        if ttl_ms <= 0 {
            ttl_ms = DEFAULT_TTL_MILLIS;
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

    pub fn from_context_args(args: &RedisArgs) -> Result<Self> {
        if let Some(dbfilename) = args.dbfilename.as_deref() {
            let mut file_path = PathBuf::from(&args.dir);
            file_path.push(dbfilename);
            if !file_path.exists() {
                bail!("RDB file is not exists. filepath: {}", file_path.display());
            }
            let file = File::open(&file_path)
                .with_context(|| format!("can not open RDB file: {}", file_path.display()))?;
            return Self::from_rdb(file);
        }
        Ok(Self::new())
    }

    pub fn from_rdb<R: Read + Seek>(r: R) -> Result<Self> {
        let db = Self::new();
        let mut parser = RdbParser::new(r);
        println!("start load rdb!");
        parser.parse(|rdb_item| {
            match rdb_item {
                RdbItem::Header { redis_ver } => {
                    println!("current rdb redis version is: {}", redis_ver)
                }
                RdbItem::Meta { key, value } => {
                    println!("metadata key: {}, value: {}", key, value)
                }
                RdbItem::DbIndex(i) => {
                    println!("current database index is: {}", i)
                }
                RdbItem::HashTableSize {
                    hash_size,
                    expire_size,
                } => {
                    println!(
                        "hashtable size: {}, expired size: {}",
                        hash_size, expire_size
                    )
                }
                RdbItem::HashTableItem {
                    key,
                    value,
                    expire_at_ms,
                } => {
                    let cur_timestamp = Utc::now().timestamp_millis();
                    let ttl_ms = match expire_at_ms {
                        Some(expire_timestamp_ms) => (expire_timestamp_ms as i64) - cur_timestamp,
                        None => DEFAULT_TTL_MILLIS, // 默认过期时间
                    };

                    if ttl_ms <= 0 {
                        return Ok(());
                    }
                    match value {
                        RdbValue::String(bytes) => db.set_with_ttl(key, bytes, ttl_ms),
                    }
                }
                RdbItem::Checksum(num) => {
                    println!("current database checksum is: {}", num)
                }
            }
            Ok(())
        })?;

        Ok(db)
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

    pub fn strlen(&self, key: &Bytes) -> Result<i64> {
        if let Some(bytes) = self.get(key)? {
            return Ok(bytes.len() as i64);
        }
        Ok(0)
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
        self.set_with_ttl(key, bytes, DEFAULT_TTL_MILLIS);
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
        item.expire_timestamp_ms = ExpirationTimestamp::from_ttl(ttl_ms);
    }

    pub fn keys(&self, pattern: String) -> Vec<Bytes> {
        if pattern.as_str() == "*" {
            self.map.iter().map(|e| e.key().clone()).collect()
        } else {
            Vec::new()
        }
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
            if self.try_xread_without_empty(&stream_keys, &entry_ids, &mut results)? {
                // 退出前注销等待信号，避免过期信号残留占用后续XADD的通知开销
                self.unregister_signal(&stream_keys, &signal)?;
                return Ok(Some(results));
            }

            let is_notified = signal.wait_until(expr_at);

            if !is_notified {
                self.unregister_signal(&stream_keys, &signal)?;
                // 超时
                if self.try_xread_without_empty(&stream_keys, &entry_ids, &mut results)? {
                    return Ok(Some(results));
                }
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

    fn unregister_signal(&self, stream_keys: &[Bytes], signal: &Arc<Signal>) -> Result<()> {
        for stream_key in stream_keys {
            let stream_arc = self.get_or_create_stream(stream_key.clone())?;
            stream_arc.unregister_waiter(Arc::clone(signal));
        }
        Ok(())
    }

    fn try_xread_without_empty(
        &self,
        stream_keys: &[Bytes],
        entry_ids: &[Bytes],
        results: &mut Vec<(Bytes, Vec<StreamEntry>)>,
    ) -> Result<bool> {
        let mut found = false;
        for (stream_key, id) in stream_keys.iter().zip(entry_ids) {
            let stream_arc = self.get_or_create_stream(stream_key.clone())?;
            let entrys = stream_arc.xread(id)?;
            if !entrys.is_empty() {
                results.push((stream_key.clone(), entrys));
                found = true;
            }
        }
        Ok(found)
    }
}

/// zset
impl MemoryDB {
    fn get_or_create_zset(&self, key: Bytes) -> Result<Arc<SortedSet>> {
        self.map
            .entry(key.clone())
            .or_insert_with(|| MemoItem::new(RedisObject::new_zset(key)))
            .value()
            .object
            .to_zset()
    }

    fn get_zset(&self, key: &Bytes) -> Result<Option<Arc<SortedSet>>> {
        self.map
            .get(key)
            .map(|entry| entry.value().object.to_zset())
            .transpose()
    }

    pub fn zadd(&self, set_key: Bytes, pairs: Vec<(f64, Bytes)>) -> Result<i64> {
        let zset = self.get_or_create_zset(set_key)?;
        let cnt = zset.add(pairs);
        Ok(cnt)
    }

    pub fn zrem(&self, set_key: Bytes, member: Bytes) -> Result<i64> {
        if let Some(zset) = self.get_zset(&set_key)? {
            let cnt = zset.remove(member);
            return Ok(cnt);
        }
        Ok(0)
    }

    pub fn zrank(&self, set_key: Bytes, member: Bytes) -> Result<Option<i64>> {
        if let Some(zset) = self.get_zset(&set_key)? {
            let rank = zset.rank(member);
            return Ok(rank);
        }
        Ok(None)
    }

    pub fn zrange(&self, set_key: Bytes, start: i64, end: i64) -> Result<Vec<ZSetEntry>> {
        if let Some(zset) = self.get_zset(&set_key)? {
            let range = zset.range(start, end);
            return Ok(range);
        }
        Ok(Vec::new())
    }

    pub fn zcard(&self, set_key: Bytes) -> Result<i64> {
        if let Some(zset) = self.get_zset(&set_key)? {
            let len = zset.len();
            return Ok(len);
        }
        Ok(0)
    }

    pub fn zscore(&self, set_key: Bytes, member: Bytes) -> Result<Option<f64>> {
        if let Some(zset) = self.get_zset(&set_key)? {
            let score_opt = zset.score(member);
            return Ok(score_opt);
        }
        return Ok(None);
    }
}

/// bitmap
impl MemoryDB {
    pub fn set_bit(&self, key: Bytes, offset: usize, bit_val: u8) -> Result<u8> {
        let old_val = match self.map.entry(key) {
            dashmap::Entry::Occupied(mut e) => {
                let item = e.get_mut();
                match &mut item.object {
                    RedisObject::String(bytes) => {
                        let old_bytes = std::mem::take(bytes);

                        let mut mut_buf = old_bytes
                            .try_into_mut()
                            .unwrap_or_else(|b| BytesMut::from(b.as_ref()));

                        let old_val = bitmap::set_bit(&mut mut_buf, offset, bit_val)?;
                        *bytes = mut_buf.freeze();
                        old_val
                    }
                    _ => bail!("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            dashmap::Entry::Vacant(e) => {
                let mut mut_buf = BytesMut::new();
                bitmap::set_bit(&mut mut_buf, offset, bit_val)?;
                let item = MemoItem::new(RedisObject::String(mut_buf.freeze()));
                e.insert(item);
                0
            }
        };
        Ok(old_val)
    }

    pub fn get_bit(&self, key: Bytes, offset: usize) -> Result<u8> {
        if let Some(bytes) = self.get(&key)? {
            let val = bitmap::get_bit(bytes, offset);
            return Ok(val);
        }
        Ok(0)
    }

    pub fn bit_count(&self, key: Bytes, start: i64, end: i64) -> Result<i64> {
        if let Some(bytes) = self.get(&key)? {
            let cnt = bitmap::bit_count(bytes, start, end);
            return Ok(cnt);
        }
        Ok(0)
    }

    pub fn bitop_binary(&self, op: String, dest: Bytes, key1: Bytes, key2: Bytes) -> Result<i64> {
        let bytes1 = self.get(&key1)?.unwrap_or_else(|| Bytes::new());
        let bytes2 = self.get(&key2)?.unwrap_or_else(|| Bytes::new());
        if bytes1.len() == 0 && bytes2.len() == 0 {
            return Ok(0);
        }
        let bytes_dest = match op.to_uppercase().as_str() {
            "AND" => bitmap::and(bytes1, bytes2),
            "OR" => bitmap::or(bytes1, bytes2),
            _ => bail!("unsupported bitop: {}", op),
        };
        let len = bytes_dest.len();
        self.set(dest, bytes_dest);
        return Ok(len as i64);
    }
}

impl MemoryDB {
    fn check_coord(&self, longitude: f64, latitude: f64) -> Result<()> {
        if longitude < -180_f64
            || longitude > 180_f64
            || latitude < -85.05112878_f64
            || latitude > 85.05112878_f64
        {
            bail!(
                "ERR invalid longitude,latitude pair {},{}",
                longitude,
                latitude
            )
        }
        Ok(())
    }

    fn check_score(&self, place: Bytes, score: f64) -> Result<()> {
        if score < 0.0 || score > u64::MAX as f64 {
            bail!(
                "ERR the score of member: {:?} store in sorted set is overflow!",
                place
            );
        }
        Ok(())
    }

    pub fn geoadd(&self, key: Bytes, longitude: f64, latitude: f64, place: Bytes) -> Result<i64> {
        self.check_coord(longitude, latitude)?;
        let score = geo::encode(latitude, longitude) as f64; // u64可直接转f64
        let cnt = self.zadd(key, vec![(score, place)])?;
        Ok(cnt)
    }

    pub fn geopos(&self, key: Bytes, places: Vec<Bytes>) -> Result<Vec<Option<Coordinates>>> {
        let len = places.len();
        if let Some(zset) = self.get_zset(&key)? {
            let mut list = Vec::with_capacity(len);
            for place in places {
                if let Some(score) = zset.score(place.clone()) {
                    self.check_score(place, score)?;
                    let coord = geo::decode(score as u64);
                    list.push(Some(coord));
                } else {
                    list.push(None);
                }
            }
            return Ok(list);
        }

        return Ok((0..len).map(|_| None).collect());
    }

    pub fn geodist(&self, key: Bytes, place1: Bytes, place2: Bytes) -> Result<f64> {
        let score1 = self
            .zscore(key.clone(), place1.clone())?
            .ok_or_else(|| anyhow!("ERR sorted set:{:?} missing member: {:?}", key, place1))?;
        self.check_score(place1, score1)?;
        let score2 = self
            .zscore(key.clone(), place2.clone())?
            .ok_or_else(|| anyhow!("ERR sorted set:{:?} missing member: {:?}", key, place2))?;
        self.check_score(place2, score2)?;

        let coord1 = geo::decode(score1 as u64);
        let coord2 = geo::decode(score2 as u64);
        let dist = geo::haversine(&coord1, &coord2);
        Ok(dist)
    }

    // 仅支持以半径(m)进行搜索
    pub fn geosearch(
        &self,
        key: Bytes,
        longitude: f64,
        latitude: f64,
        radius: f64,
    ) -> Result<Vec<Bytes>> {
        let mut ans = Vec::with_capacity(16);
        let entrys = self.zrange(key, 0, -1)?;
        let src_coord = Coordinates::new(longitude, latitude);
        for entry in entrys {
            self.check_score(entry.member.clone(), entry.score)?;
            let dest_coord: Coordinates = geo::decode(entry.score as u64);
            let dist = geo::haversine(&src_coord, &dest_coord);
            if dist <= radius {
                ans.push(entry.member);
            }
        }
        Ok(ans)
    }
}
