use super::ReList;
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::Arc;

#[derive(Debug)]
pub enum RedisObject {
    String(Bytes),
    List(Arc<ReList>),
}

impl RedisObject {
    pub fn new_list(name: Bytes) -> Self {
        Self::List(Arc::new(ReList::new(name)))
    }

    pub fn obj_type(&self) -> String {
        match self {
            RedisObject::String(_) => "string".into(),
            RedisObject::List(_) => "list".into(),
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
            RedisObject::List(list_rc) => Ok(Arc::clone(list_rc)),
            _ => bail!("type is not list"),
        }
    }
}
