use super::{ReList, ReStream};
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::Arc;

#[derive(Debug)]
pub enum RedisObject {
    String(Bytes),
    List(Arc<ReList>),
    Stream(Arc<ReStream>),
}

impl RedisObject {
    pub fn new_list(name: Bytes) -> Self {
        Self::List(Arc::new(ReList::new(name)))
    }

    pub fn new_stream(name: Bytes) -> Self {
        Self::Stream(Arc::new(ReStream::new(name)))
    }

    pub fn obj_type(&self) -> String {
        match self {
            RedisObject::String(_) => "string".into(),
            RedisObject::List(_) => "list".into(),
            RedisObject::Stream(_) => "stream".into(),
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
}
