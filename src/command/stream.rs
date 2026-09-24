use crate::{CommandResponse, Context, Value};
use anyhow::bail;
use anyhow::{Ok, Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;
use std::vec::IntoIter;

pub(super) fn execute_xadd(
    mut arg_iter: IntoIter<Value>,
    ctx: &Arc<Context>,
) -> Result<CommandResponse> {
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
    let db = ctx.db_ref();
    let id = db.xadd(stream_key, entry_id, args)?;
    Ok(id.into())
}

pub(super) fn execute_xrange(
    mut arg_iter: IntoIter<Value>,
    ctx: &Arc<Context>,
) -> Result<CommandResponse> {
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
    let db = ctx.db_ref();
    let result = match db.xrange(stream_key, start, end)? {
        Some(entrys) => entrys.into(),
        _ => Value::EmptyArrays,
    };
    Ok(result.into())
}

pub(super) fn execute_xread(
    mut arg_iter: IntoIter<Value>,
    ctx: &Arc<Context>,
) -> Result<CommandResponse> {
    let Some(arg1) = arg_iter.next() else {
        bail!("xread format error")
    };
    let mut timeout_in_milli: Option<i64> = None;
    if &arg1.into_string()?.to_uppercase() == "BLOCK" {
        let arg2 = arg_iter
            .next()
            .ok_or_else(|| anyhow!("xread command need timeout argument in block mode!"))?;
        timeout_in_milli = Some(arg2.into_integer()?);
        arg_iter.next(); // streams
    }

    let args = arg_iter
        .map(|v| v.into_bulk_bytes())
        .collect::<Result<Vec<Bytes>>>()?;
    let args_len = args.len();
    let db = ctx.db_ref();
    let (stream_keys, ids) = args.split_at(args_len / 2);
    let db_result = if let Some(t) = timeout_in_milli {
        db.block_xread(stream_keys.to_vec(), ids.to_vec(), t)?
    } else {
        db.xread(stream_keys, ids)?
    };

    let result = match db_result {
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
        None => {
            if timeout_in_milli.is_none() {
                Value::EmptyArrays
            } else {
                Value::NullArrays
            }
        }
    };
    Ok(result.into())
}
