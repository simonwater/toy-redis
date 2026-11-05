use crate::{Command, CommandResponse, Context, Value};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;

pub(super) fn execute_zadd(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let set_key = cmd
        .args
        .iter()
        .next()
        .ok_or_else(|| anyhow!("zadd command missing sorted set key!"))?
        .to_bulk_bytes()?;

    let chunks = cmd.args[1..].chunks_exact(2);
    let mut args = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let score = chunk[0].to_double()?;
        let member = chunk[1].to_bulk_bytes()?;
        args.push((score, member));
    }

    let db = ctx.db_ref();
    let len = db.zadd(set_key, args)?;
    Ok(len.into())
}

pub(super) fn execute_zrank(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let set_key = args_iter
        .next()
        .ok_or_else(|| anyhow!("zrank command missing sorted set key!"))?
        .to_bulk_bytes()?;
    let member = args_iter
        .next()
        .ok_or_else(|| anyhow!("zrank command missing member argument!"))?
        .to_bulk_bytes()?;
    let db = ctx.db_ref();
    let val = if let Some(rank) = db.zrank(set_key, member)? {
        rank.into()
    } else {
        Value::NullBulkStrings
    };
    return Ok(val.into());
}

pub(super) fn execute_zrange(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let set_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("zrange command missing sorted set key!"))?
        .to_bulk_bytes()?;
    let start = arg_iter
        .next()
        .ok_or_else(|| anyhow!("zrange command missing start index."))?
        .to_integer()?;
    let end = arg_iter
        .next()
        .ok_or_else(|| anyhow!("zrange command missing end index."))?
        .to_integer()?;

    let db = ctx.db_ref();
    let value = db.zrange(set_key, start, end)?;
    Ok(value.into())
}

pub(super) fn execute_zcard(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let set_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("zcard command missing sorted set key!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let len = db.zcard(set_key)?;
    Ok(len.into())
}

pub(super) fn execute_zscore(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let set_key = args_iter
        .next()
        .ok_or_else(|| anyhow!("zscore command missing sorted set key!"))?
        .to_bulk_bytes()?;
    let member = args_iter
        .next()
        .ok_or_else(|| anyhow!("zscore command missing member argument!"))?
        .to_bulk_bytes()?;
    let db = ctx.db_ref();
    let val = if let Some(score) = db.zscore(set_key, member)? {
        Bytes::from(score.to_string()).into()
    } else {
        Value::NullBulkStrings
    };
    return Ok(val.into());
}

pub(super) fn execute_zrem(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let set_key = args_iter
        .next()
        .ok_or_else(|| anyhow!("zrem command missing sorted set key!"))?
        .to_bulk_bytes()?;
    let member = args_iter
        .next()
        .ok_or_else(|| anyhow!("zrem command missing member argument!"))?
        .to_bulk_bytes()?;
    let db = ctx.db_ref();
    let cnt = db.zrem(set_key, member)?;
    return Ok(cnt.into());
}
