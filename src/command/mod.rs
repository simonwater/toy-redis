use crate::Value;
use crate::{MemoryDB, memory_db};
use anyhow::{Result, bail};
use std::sync::{Arc, RwLock};
use std::vec::IntoIter;

pub enum Command {
    Ping,
    Echo(IntoIter<Value>),
    Set(IntoIter<Value>),
    Get(IntoIter<Value>),
}

impl Command {
    pub fn new(cmd_values: Vec<Value>) -> Result<Self> {
        let mut cmd_iter: std::vec::IntoIter<Value> = cmd_values.into_iter();
        let Some(name_value) = cmd_iter.next() else {
            bail!("missing command!");
        };

        match name_value {
            Value::BulkStrings(s) | Value::SimpleStrings(s) => {
                let name = s.to_uppercase();
                let cmd = match name.as_str() {
                    "PING" => Command::Ping,
                    "ECHO" => Command::Echo(cmd_iter),
                    "GET" => Command::Get(cmd_iter),
                    "SET" => Command::Set(cmd_iter),
                    _ => bail!("unsupported command!"),
                };
                return Ok(cmd);
            }
            _ => {
                bail!("command format error!");
            }
        }
    }

    pub fn execute(self, db: &Arc<RwLock<MemoryDB>>) -> Result<Value> {
        match self {
            Command::Ping => Ok(Value::SimpleStrings("PONG".into())),
            Command::Echo(args) => execute_echo(args),
            Command::Set(args) => execute_set(args, db),
            Command::Get(args) => execute_get(args, db),
        }
    }
}

fn execute_echo(mut arg_iter: IntoIter<Value>) -> Result<Value> {
    let arg = arg_iter.next();
    if arg.is_none() {
        return Ok(Value::SimpleErrors("echo command missing argument!".into()));
    }
    return Ok(arg.unwrap().clone());
}

fn execute_set(mut arg_iter: IntoIter<Value>, db: &Arc<RwLock<MemoryDB>>) -> Result<Value> {
    let Some(key) = arg_iter.next() else {
        return Ok(Value::SimpleErrors("set command missing key!".into()));
    };
    let Some(val) = arg_iter.next() else {
        return Ok(Value::SimpleErrors("set command missing value!".into()));
    };
    let mut ttl_ms = memory_db::DAY_IN_MILLIS;
    if let (Some(f), Some(ttl)) = (arg_iter.next(), arg_iter.next()) {
        if let (Ok(f), Ok(ttl)) = (f.into_string(), ttl.into_integer()) {
            ttl_ms = match f.to_uppercase().as_str() {
                "EX" => 1000 * ttl,
                "PX" => ttl,
                _ => memory_db::DAY_IN_MILLIS,
            };
        }
    }

    match key {
        Value::SimpleStrings(k) | Value::BulkStrings(k) => {
            db.write().unwrap().set_with_ttl(k, val, ttl_ms);
            return Ok(Value::SimpleStrings("OK".into()));
        }
        _ => return Ok(Value::SimpleStrings("unsupported key type!".into())),
    }
}

fn execute_get(mut arg_iter: IntoIter<Value>, db: &Arc<RwLock<MemoryDB>>) -> Result<Value> {
    let Some(key) = arg_iter.next() else {
        return Ok(Value::SimpleErrors("set command missing key!".into()));
    };
    match key {
        Value::SimpleStrings(k) | Value::BulkStrings(k) => {
            let value = db
                .read()
                .unwrap()
                .get(&k)
                .cloned()
                .unwrap_or_else(|| Value::NullBulkStrings);
            return Ok(value);
        }
        _ => return Ok(Value::SimpleStrings("unsupported key type!".into())),
    }
}
