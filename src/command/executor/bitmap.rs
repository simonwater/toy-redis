use crate::{Command, CommandResponse, Context, Value};
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub(super) fn execute_setbit(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("setbit command missing bit map key!"))?
        .to_bulk_bytes()?;
    let offset: usize = arg_iter
        .next()
        .ok_or_else(|| anyhow!("setbit command missing offset argument!"))?
        .to_integer()?
        .try_into()?;
    let bit_val: u8 = arg_iter
        .next()
        .ok_or_else(|| anyhow!("setbit command missing bit value argument!"))?
        .to_integer()?
        .try_into()?;

    let db = ctx.db_ref();
    let old_val = db.set_bit(key, offset, bit_val)? as i64;
    Ok(old_val.into())
}

pub(super) fn execute_getbit(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("getbit command missing bit map key!"))?
        .to_bulk_bytes()?;
    let offset: usize = arg_iter
        .next()
        .ok_or_else(|| anyhow!("getbit command missing offset argument!"))?
        .to_integer()?
        .try_into()?;

    let db = ctx.db_ref();
    let val = db.get_bit(key, offset)? as i64;
    Ok(val.into())
}

pub(super) fn execute_bitcount(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("bitcount command missing bit map key!"))?
        .to_bulk_bytes()?;
    let start = arg_iter
        .next()
        .unwrap_or_else(|| &Value::Integer(0))
        .to_integer()?;
    let end = arg_iter
        .next()
        .unwrap_or_else(|| &Value::Integer(-1))
        .to_integer()?;

    let db = ctx.db_ref();
    let cnt = db.bit_count(key, start, end)?;
    Ok(cnt.into())
}

pub(super) fn execute_bitop(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let op = arg_iter
        .next()
        .ok_or_else(|| anyhow!("bitop command missing operator argument!"))?
        .to_string()?;
    let dest = arg_iter
        .next()
        .ok_or_else(|| anyhow!("bitop command missing destination key!"))?
        .to_bulk_bytes()?;
    let key1 = arg_iter
        .next()
        .ok_or_else(|| anyhow!("bitop command missing source key!"))?
        .to_bulk_bytes()?;
    let key2 = arg_iter
        .next()
        .ok_or_else(|| anyhow!("bitop command needs two source keys!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let cnt = db.bitop_binary(op, dest, key1, key2)?;
    Ok(cnt.into())
}
