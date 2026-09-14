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

pub(super) fn execute_xread(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    arg_iter.next(); // "STREAMS"
    let args = arg_iter
        .map(|v| v.into_bulk_bytes())
        .collect::<Result<Vec<Bytes>>>()?;
    let (stream_keys, ids) = args.split_at(args.len() / 2);
    let result = match db.xread(stream_keys, ids)? {
        Some(pairs) => {
            let mut frames: Vec<Value> = Vec::with_capacity(pairs.len());
            for (stream_key, entrys) in pairs {
                let mut frame: Vec<Value> = Vec::with_capacity(2);
                frame.push(Value::BulkStrings(stream_key));
                frame.push(entrys.into());

                frames.push(Value::Arrays(frame));
            }
            Value::Arrays(frames)
        }
        _ => Value::EmptyArrays,
    };
    Ok(result)
}
