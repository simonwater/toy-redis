use crate::{Command, CommandResponse, Context, Value};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;

pub(super) fn execute_rpush(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("rpush command missing list key!"))?
        .to_bulk_bytes()?;
    let args = arg_iter
        .map(|v| v.to_bulk_bytes())
        .collect::<Result<Vec<Bytes>>>()?;

    let db = ctx.db_ref();
    let len = db.rpush(list_key, args)?;
    Ok(len.into())
}

pub(super) fn execute_lpush(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lpush command missing list key!"))?
        .to_bulk_bytes()?;
    let args = arg_iter
        .map(|v| v.to_bulk_bytes())
        .collect::<Result<Vec<Bytes>>>()?;

    let db = ctx.db_ref();
    let len = db.lpush(list_key, args)?;
    Ok(len.into())
}

pub(super) fn execute_lrange(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lrange command missing list key!"))?
        .to_bulk_bytes()?;
    let start = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lrange command missing start index."))?
        .to_integer()?;
    let end = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lrange command missing end index."))?
        .to_integer()?;

    let db = ctx.db_ref();
    let value = match db.lrange(list_key, start, end)? {
        Some(bytes_vec) => bytes_vec.into(),
        _ => Value::EmptyArrays,
    };
    Ok(value.into())
}

pub(super) fn execute_llen(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("llen command missing list key!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let len = db.llen(list_key)?;
    Ok(len.into())
}

pub(super) fn execute_lpop(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lpop command missing list key!"))?
        .to_bulk_bytes()?;
    let cnt = arg_iter
        .next()
        .unwrap_or_else(|| &Value::Integer(1))
        .to_integer()?;

    let db = ctx.db_ref();
    let value = match db.lpop(list_key, cnt)? {
        Some(bytes_vec) => {
            if cnt == 1 {
                bytes_vec[0].clone().into()
            } else {
                bytes_vec.into()
            }
        }
        _ => Value::NullBulkStrings,
    };
    Ok(value.into())
}

pub(super) fn execute_rpop(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("rpop command missing list key!"))?
        .to_bulk_bytes()?;
    let cnt = arg_iter
        .next()
        .unwrap_or_else(|| &Value::Integer(1))
        .to_integer()?;

    let db = ctx.db_ref();
    let value = match db.rpop(list_key, cnt)? {
        Some(bytes_vec) => {
            if cnt == 1 {
                Value::BulkStrings(bytes_vec[0].clone())
            } else {
                bytes_vec.into()
            }
        }
        _ => Value::NullBulkStrings,
    };
    Ok(value.into())
}

pub(super) fn execute_blpop(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("blpop command missing list key!"))?
        .to_bulk_bytes()?;
    let timeout = arg_iter
        .next()
        .unwrap_or_else(|| &Value::Integer(1))
        .to_double()?;

    let db = ctx.db_ref();
    let value = match db.blpop(list_key.clone(), timeout)? {
        Some(bytes) => {
            let bytes_vec = vec![list_key.clone(), bytes];
            bytes_vec.into()
        }
        _ => Value::NullArrays,
    };
    Ok(value.into())
}

pub(super) fn execute_brpop(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("brpop command missing list key!"))?
        .to_bulk_bytes()?;
    let timeout = arg_iter
        .next()
        .unwrap_or_else(|| &Value::Integer(1))
        .to_double()?;

    let db = ctx.db_ref();
    let value = match db.brpop(list_key.clone(), timeout)? {
        Some(bytes) => {
            let bytes_vec = vec![list_key.clone(), bytes];
            bytes_vec.into()
        }
        _ => Value::NullArrays,
    };
    Ok(value.into())
}
