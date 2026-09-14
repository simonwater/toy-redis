mod list;
mod stream;

use crate::Value;
use crate::{MemoryDB, db};
use anyhow::{Result, anyhow, bail};
use std::sync::Arc;
use std::vec::IntoIter;

pub enum Command {
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
}

impl Command {
    pub fn new(cmd_values: Vec<Value>) -> Result<Self> {
        let mut cmd_iter: std::vec::IntoIter<Value> = cmd_values.into_iter();
        let cmd = cmd_iter.next().ok_or_else(|| anyhow!("missing command!"))?;
        match cmd {
            Value::BulkStrings(s) => {
                let name = String::from_utf8(s.to_vec())?;
                let name = name.to_uppercase();
                let cmd = match name.as_str() {
                    "PING" => Command::Ping(cmd_iter),
                    "ECHO" => Command::Echo(cmd_iter),
                    "GET" => Command::Get(cmd_iter),
                    "SET" => Command::Set(cmd_iter),
                    "RPUSH" => Command::Rpush(cmd_iter),
                    "LPUSH" => Command::Lpush(cmd_iter),
                    "RPOP" => Command::Rpop(cmd_iter),
                    "LPOP" => Command::Lpop(cmd_iter),
                    "BRPOP" => Command::Brpop(cmd_iter),
                    "BLPOP" => Command::Blpop(cmd_iter),
                    "LRANGE" => Command::Lrange(cmd_iter),
                    "LLEN" => Command::Llen(cmd_iter),
                    "TYPE" => Command::Type(cmd_iter),
                    "XADD" => Command::Xadd(cmd_iter),
                    "XRANGE" => Command::Xrange(cmd_iter),
                    "XREAD" => Command::Xread(cmd_iter),
                    _ => bail!("unsupported command!"),
                };
                return Ok(cmd);
            }
            _ => {
                bail!("command format error!");
            }
        }
    }

    pub fn execute(self, db: &Arc<MemoryDB>) -> Result<Value> {
        match self {
            Command::Ping(args) => execute_ping(args, db),
            Command::Echo(args) => execute_echo(args, db),
            Command::Set(args) => execute_set(args, db),
            Command::Get(args) => execute_get(args, db),
            Command::Type(args) => execute_type(args, db),
            Command::Rpush(args) => list::execute_rpush(args, db),
            Command::Lpush(args) => list::execute_lpush(args, db),
            Command::Rpop(args) => list::execute_rpop(args, db),
            Command::Lpop(args) => list::execute_lpop(args, db),
            Command::Brpop(args) => list::execute_brpop(args, db),
            Command::Blpop(args) => list::execute_blpop(args, db),
            Command::Lrange(args) => list::execute_lrange(args, db),
            Command::Llen(args) => list::execute_llen(args, db),
            Command::Xadd(args) => stream::execute_xadd(args, db),
            Command::Xrange(args) => stream::execute_xrange(args, db),
            Command::Xread(args) => stream::execute_xread(args, db),
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
