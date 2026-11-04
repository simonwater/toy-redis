use super::{ReList, ReStream, SortedSet};
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::Arc;

#[derive(Debug)]
pub enum RedisObject {
    String(Bytes),
    List(Arc<ReList>),
    Stream(Arc<ReStream>),
    SortedSet(Arc<SortedSet>),
}

impl RedisObject {
    pub fn new_integer(val: i64) -> Self {
        Self::String(Bytes::from(val.to_string()))
    }

    pub fn new_list(name: Bytes) -> Self {
        Self::List(Arc::new(ReList::new(name)))
    }

    pub fn new_stream(name: Bytes) -> Self {
        Self::Stream(Arc::new(ReStream::new(name)))
    }

    pub fn new_zset(_name: Bytes) -> Self {
        Self::SortedSet(Arc::new(SortedSet::new()))
    }

    pub fn obj_type(&self) -> String {
        match self {
            RedisObject::String(_) => "string".into(),
            RedisObject::List(_) => "list".into(),
            RedisObject::Stream(_) => "stream".into(),
            RedisObject::SortedSet(_) => "sorted set".into(),
        }
    }

    pub fn to_bulk_bytes(&self) -> Result<Bytes> {
        match self {
            RedisObject::String(bytes) => Ok(bytes.clone()),
            _ => bail!("type is not bulk strings"),
        }
    }

    pub fn to_relist(&self) -> Result<Arc<ReList>> {
        match self {
            RedisObject::List(list_arc) => Ok(Arc::clone(list_arc)),
            _ => bail!("type is not list"),
        }
    }

    pub fn to_restream(&self) -> Result<Arc<ReStream>> {
        match self {
            RedisObject::Stream(stream_arc) => Ok(Arc::clone(stream_arc)),
            _ => bail!("type is not stream"),
        }
    }

    pub fn to_zset(&self) -> Result<Arc<SortedSet>> {
        match self {
            RedisObject::SortedSet(zset_arc) => Ok(Arc::clone(zset_arc)),
            _ => bail!("type is not sorted set"),
        }
    }
}
