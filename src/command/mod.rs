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
}

impl Command {
    pub fn new(cmd_values: Vec<Value>) -> Result<Self> {
        let mut cmd_iter: std::vec::IntoIter<Value> = cmd_values.into_iter();
        let name_value = cmd_iter.next().ok_or_else(|| anyhow!("missing command!"))?;

        match name_value {
            Value::BulkStrings(s) | Value::SimpleStrings(s) => {
                let name = s.to_uppercase();
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
            Command::Rpush(args) => execute_rpush(args, db),
            Command::Lpush(args) => execute_lpush(args, db),
            Command::Rpop(args) => execute_rpop(args, db),
            Command::Lpop(args) => execute_lpop(args, db),
            Command::Brpop(args) => execute_brpop(args, db),
            Command::Blpop(args) => execute_blpop(args, db),
            Command::Lrange(args) => execute_lrange(args, db),
            Command::Llen(args) => execute_llen(args, db),
            Command::Type(args) => execute_type(args, db),
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
        .ok_or_else(|| anyhow!("set command missing key!"))?;
    let val = arg_iter
        .next()
        .ok_or_else(|| anyhow!("set command missing value!"))?;
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

    match key {
        Value::SimpleStrings(k) | Value::BulkStrings(k) => {
            db.set_with_ttl(k, val, ttl_ms);
            return Ok(Value::SimpleStrings("OK".into()));
        }
        _ => bail!("unsupported key type!"),
    }
}

fn execute_get(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("get command missing key!"))?;
    match key {
        Value::SimpleStrings(k) | Value::BulkStrings(k) => {
            let value = db.get(&k).unwrap_or_else(|| Value::NullBulkStrings);
            return Ok(value);
        }
        _ => bail!("unsupported key type!"),
    }
}

fn execute_rpush(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("rpush command missing list key!"))?;
    let list_key = list_key.into_string()?;
    let len = db.rpush(list_key, arg_iter);
    Ok(len)
}

fn execute_lpush(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lpush command missing list key!"))?;
    let list_key = list_key.into_string()?;
    let len = db.lpush(list_key, arg_iter);
    Ok(len)
}

fn execute_lrange(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lrange command missing list key!"))?
        .into_string()?;
    let start = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lrange command missing start index."))?
        .into_integer()?;
    let end = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lrange command missing end index."))?
        .into_integer()?;
    let vals = db.lrange(list_key, start, end);
    Ok(vals)
}

fn execute_llen(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("llen command missing list key!"))?
        .into_string()?;
    let len = db.llen(list_key);
    Ok(len)
}

fn execute_lpop(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("lpop command missing list key!"))?
        .into_string()?;
    let cnt = arg_iter
        .next()
        .unwrap_or_else(|| Value::Integer(1))
        .into_integer()?;
    let val = db.lpop(list_key, cnt);
    Ok(val)
}

fn execute_rpop(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("rpop command missing list key!"))?
        .into_string()?;
    let cnt = arg_iter
        .next()
        .unwrap_or_else(|| Value::Integer(1))
        .into_integer()?;
    let val = db.rpop(list_key, cnt);
    Ok(val)
}

fn execute_blpop(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("blpop command missing list key!"))?
        .into_string()?;
    let timeout = arg_iter
        .next()
        .unwrap_or_else(|| Value::Integer(1))
        .into_double()?;
    let val = db.blpop(list_key, timeout);
    Ok(val)
}

fn execute_brpop(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let list_key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("brpop command missing list key!"))?
        .into_string()?;
    let timeout = arg_iter
        .next()
        .unwrap_or_else(|| Value::Integer(1))
        .into_double()?;
    let val = db.brpop(list_key, timeout);
    Ok(val)
}

fn execute_type(mut arg_iter: IntoIter<Value>, db: &Arc<MemoryDB>) -> Result<Value> {
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("type command missing key!"))?
        .into_string()?;
    if let Some(_) = db.get(&key) {
        return Ok(Value::SimpleStrings("string".into()));
    }
    return Ok(Value::SimpleStrings("none".into()));
}
