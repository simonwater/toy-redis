use crate::MemoryDB;
use crate::Value;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;
use std::vec::IntoIter;

pub(super) fn execute_xadd(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let stream_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("xadd command missing key!"))?
        .into_bulk_bytes()?;
    let entry_id = arg_iter
        .next()
        .ok_or_else(|| anyhow!("xadd command missing entry id!"))?
        .into_bulk_bytes()?;
    let args = arg_iter
        .map(|v| v.into_bulk_bytes())
        .collect::<Result<Vec<Bytes>>>()?;
    let id = db.xadd(stream_key, entry_id, args)?;
    Ok(Value::BulkStrings(id))
}

pub(super) fn execute_xrange(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let stream_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("xrange command missing key!"))?
        .into_bulk_bytes()?;
    let start = arg_iter
        .next()
        .ok_or_else(|| anyhow!("xrange command missing start entry id!"))?
        .into_bulk_bytes()?;
    let end = arg_iter
        .next()
        .ok_or_else(|| anyhow!("xrange command missing end entry id!"))?
        .into_bulk_bytes()?;
    let result = match db.xrange(stream_key, start, end)? {
        Some(entrys) => entrys.into(),
        _ => Value::EmptyArrays,
    };
    Ok(result)
}
