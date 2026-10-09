use super::CommandResponse;
use crate::info::SectionType;
use crate::{Command, Context, Value, db};
use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use std::io::Cursor;
use std::sync::Arc;

pub(super) fn execute_ping(_cmd: &Command, _ctx: &Arc<Context>) -> Result<CommandResponse> {
    Ok("PONG".into())
}

pub(super) fn execute_echo(cmd: &Command, _ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let arg = arg_iter.next();
    if arg.is_none() {
        bail!("echo command missing argument!")
    }
    return Ok(arg.unwrap().clone().into());
}

pub(super) fn execute_set(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("set command missing key!"))?
        .to_bulk_bytes()?;
    let val = arg_iter
        .next()
        .ok_or_else(|| anyhow!("set command missing value!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let mut ttl_ms = db::DAY_IN_MILLIS;
    if let (Some(f), Some(ttl)) = (arg_iter.next(), arg_iter.next()) {
        if let (Ok(f), Ok(ttl)) = (f.to_string(), ttl.to_integer()) {
            ttl_ms = match f.to_uppercase().as_str() {
                "EX" => 1000 * ttl,
                "PX" => ttl,
                _ => db::DAY_IN_MILLIS,
            };
        }
    }

    db.set_with_ttl(key, val, ttl_ms);
    return Ok("OK".into());
}

pub(super) fn execute_get(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("get command missing key!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let value = db.get(&key)?;
    if let Some(bytes) = value {
        return Ok(Value::BulkStrings(bytes).into());
    }
    return Ok(Value::NullBulkStrings.into());
}

pub(super) fn execute_type(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("type command missing key!"))?
        .to_bulk_bytes()?;
    let db = ctx.db_ref();
    let t = db.obj_type(&key);
    Ok(t.into())
}

pub(super) fn execute_incr(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("ERR Incr command missing key"))?
        .to_bulk_bytes()?;
    let db = ctx.db_ref();
    let result = db.incr(key)?;
    Ok(result.into())
}

pub(super) fn execute_info(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let section = arg_iter.next().map(|v| v.to_string()).transpose()?;
    let sec_type = SectionType::new(section);
    let mut out = String::with_capacity(64);
    ctx.info_ref().output(sec_type, &mut out);
    Ok(Bytes::from(out).into())
}

pub(super) fn execute_replconf(_cmd: &Command, _ctx: &Arc<Context>) -> Result<CommandResponse> {
    Ok("OK".into())
}

pub(super) fn execute_psync(_cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let master_id = ctx.info_ref().get_master_replid();

    let s: String = format!("FULLRESYNC {} 0", master_id);
    let stream = empty_rdb();
    let len = stream.len();
    let response = CommandResponse::Replication {
        init_value: s.into(),
        stream: Box::new(Cursor::new(stream)),
        len,
    };
    Ok(response)
}

pub(super) fn empty_rdb() -> Vec<u8> {
    const EMPTY_RDB_HEX: &'static str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";
    (0..EMPTY_RDB_HEX.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&EMPTY_RDB_HEX[i..i + 2], 16).unwrap())
        .collect()
}
