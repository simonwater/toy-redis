pub mod executor;

mod response;

use bytes::Bytes;
pub use response::{CommandResponse, ConnectionState};

use crate::Value;
use anyhow::{Result, anyhow, bail};

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub args: Vec<Value>,
    pub bytes: Bytes,
}

impl Command {
    pub fn new(cmd_values: Vec<Value>, bytes: Bytes) -> Result<Self> {
        let mut iter: std::vec::IntoIter<Value> = cmd_values.into_iter();
        let cmd = iter.next().ok_or_else(|| anyhow!("missing command!"))?;
        let name = match cmd {
            Value::BulkStrings(b) => String::from_utf8(b.to_vec())?.to_uppercase(),
            _ => bail!("command format error!"),
        };
        Ok(Self {
            name,
            args: iter.collect(),
            bytes,
        })
    }

    pub fn is_write(&self) -> bool {
        is_write_command(self)
    }
}

fn is_write_command(cmd: &Command) -> bool {
    match cmd.name.as_str() {
        "SET" | "INCR" | "RPUSH" | "LPUSH" | "RPOP" | "LPOP" | "BRPOP" | "BLPOP" | "XADD"
        | "WATCH" | "UNWATCH" | "MULTI" | "EXEC" | "DISCARD" => true,
        _ => false,
    }
}
