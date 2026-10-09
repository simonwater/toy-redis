pub mod executor;

mod response;
mod trans;

pub use response::{CommandResponse, ConnectionState};
pub use trans::Transaction;

use crate::Value;
use anyhow::{Result, anyhow, bail};

#[derive(Debug, Clone)]
pub struct Command {
    name: String,
    args: Vec<Value>,
}

impl Command {
    pub fn new(cmd_values: Vec<Value>) -> Result<Self> {
        let mut iter: std::vec::IntoIter<Value> = cmd_values.into_iter();
        let cmd = iter.next().ok_or_else(|| anyhow!("missing command!"))?;
        let name = match cmd {
            Value::BulkStrings(b) => String::from_utf8(b.to_vec())?.to_uppercase(),
            _ => bail!("command format error!"),
        };
        Ok(Self {
            name,
            args: iter.collect(),
        })
    }
}
