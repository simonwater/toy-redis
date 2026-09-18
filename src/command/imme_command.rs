use super::{Transaction, list, stream};
use crate::Value;
use crate::{MemoryDB, db};
use anyhow::{Result, anyhow, bail};
use std::sync::Arc;
use std::vec::IntoIter;

pub enum ImmeCommand {
    Ping(IntoIter<Value>),
    Echo(IntoIter<Value>),
    Set(IntoIter<Value>),
    Get(IntoIter<Value>),
    Rpush(IntoIter<Value>),
    Lpush(IntoIter<Value>),
    Rpop(IntoIter<Value>),
    Lpop(IntoIter<Value>),
    Brpop(IntoIter<Value>),
    Blpop(IntoIter<Value>),
    Lrange(IntoIter<Value>),
    Llen(IntoIter<Value>),
    Type(IntoIter<Value>),
    Xadd(IntoIter<Value>),
    Xrange(IntoIter<Value>),
    Xread(IntoIter<Value>),
    Incr(IntoIter<Value>),
}

impl ImmeCommand {
    pub fn execute(self, db: &Arc<MemoryDB>, trans: &mut Transaction) -> Result<Value> {
        if trans.is_started() {
            return trans.add_command(self); // 延迟统一处理
        }

        match self {
            Self::Ping(args) => execute_ping(args, db),
            Self::Echo(args) => execute_echo(args, db),
            Self::Set(args) => execute_set(args, db),
            Self::Get(args) => execute_get(args, db),
            Self::Incr(args) => execute_incr(args, db),
            Self::Type(args) => execute_type(args, db),
            Self::Rpush(args) => list::execute_rpush(args, db),
            Self::Lpush(args) => list::execute_lpush(args, db),
            Self::Rpop(args) => list::execute_rpop(args, db),
            Self::Lpop(args) => list::execute_lpop(args, db),
            Self::Brpop(args) => list::execute_brpop(args, db),
            Self::Blpop(args) => list::execute_blpop(args, db),
            Self::Lrange(args) => list::execute_lrange(args, db),
            Self::Llen(args) => list::execute_llen(args, db),
            Self::Xadd(args) => stream::execute_xadd(args, db),
            Self::Xrange(args) => stream::execute_xrange(args, db),
            Self::Xread(args) => stream::execute_xread(args, db),
        }
    }
}

fn execute_ping(mut _arg_iter: IntoIter<Value>, _db: &Arc<MemoryDB>) -> Result<Value> {
    Ok(Value::SimpleStrings("PONG".into()))
}

fn execute_echo(mut arg_iter: IntoIter<Value>, _db: &Arc<MemoryDB>) -> Result<Value> {
    let arg = arg_iter.next();
    if arg.is_none() {
        bail!("echo command missing argument!")
    }
    return Ok(arg.unwrap().clone());
}

fn execute_set(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("set command missing key!"))?
        .into_bulk_bytes()?;
    let val = arg_iter
        .next()
        .ok_or_else(|| anyhow!("set command missing value!"))?
        .into_bulk_bytes()?;

    let mut ttl_ms = db::DAY_IN_MILLIS;
    if let (Some(f), Some(ttl)) = (arg_iter.next(), arg_iter.next()) {
        if let (Ok(f), Ok(ttl)) = (f.into_string(), ttl.into_integer()) {
            ttl_ms = match f.to_uppercase().as_str() {
                "EX" => 1000 * ttl,
                "PX" => ttl,
                _ => db::DAY_IN_MILLIS,
            };
        }
    }

    db.set_with_ttl(key, val, ttl_ms);
    return Ok(Value::SimpleStrings("OK".into()));
}

fn execute_get(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("get command missing key!"))?
        .into_bulk_bytes()?;

    let value = db.get(&key)?;
    if let Some(bytes) = value {
        return Ok(Value::BulkStrings(bytes));
    }
    return Ok(Value::NullBulkStrings);
}

fn execute_type(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("type command missing key!"))?
        .into_bulk_bytes()?;
    let t = db.obj_type(&key);
    Ok(Value::SimpleStrings(t))
}

fn execute_incr(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("ERR Incr command missing key"))?
        .into_bulk_bytes()?;
    let result = db.incr(key)?;
    Ok(Value::Integer(result))
}
