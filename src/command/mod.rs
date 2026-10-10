pub mod executor;

mod response;
mod trans;

use bytes::Bytes;
pub use response::{CommandResponse, ConnectionState};
pub use trans::Transaction;

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
}
