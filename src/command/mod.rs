mod imme_command;
mod list;
mod response;
mod stream;
mod trans;

pub use response::CommandResponse;
pub use trans::Transaction;

use crate::Context;
use crate::Value;
use crate::command::imme_command::ImmeCommand;
use crate::command::trans::TransCommand;
use anyhow::{Result, anyhow, bail};
use std::sync::Arc;

pub enum Command {
    ImmeCommand(ImmeCommand),
    TransCommand(TransCommand),
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
                    "PING" => Self::ImmeCommand(ImmeCommand::Ping(cmd_iter)),
                    "ECHO" => Self::ImmeCommand(ImmeCommand::Echo(cmd_iter)),
                    "INFO" => Self::ImmeCommand(ImmeCommand::Info(cmd_iter)),
                    "GET" => Self::ImmeCommand(ImmeCommand::Get(cmd_iter)),
                    "SET" => Self::ImmeCommand(ImmeCommand::Set(cmd_iter)),
                    "RPUSH" => Self::ImmeCommand(ImmeCommand::Rpush(cmd_iter)),
                    "LPUSH" => Self::ImmeCommand(ImmeCommand::Lpush(cmd_iter)),
                    "RPOP" => Self::ImmeCommand(ImmeCommand::Rpop(cmd_iter)),
                    "LPOP" => Self::ImmeCommand(ImmeCommand::Lpop(cmd_iter)),
                    "BRPOP" => Self::ImmeCommand(ImmeCommand::Brpop(cmd_iter)),
                    "BLPOP" => Self::ImmeCommand(ImmeCommand::Blpop(cmd_iter)),
                    "LRANGE" => Self::ImmeCommand(ImmeCommand::Lrange(cmd_iter)),
                    "LLEN" => Self::ImmeCommand(ImmeCommand::Llen(cmd_iter)),
                    "TYPE" => Self::ImmeCommand(ImmeCommand::Type(cmd_iter)),
                    "XADD" => Self::ImmeCommand(ImmeCommand::Xadd(cmd_iter)),
                    "XRANGE" => Self::ImmeCommand(ImmeCommand::Xrange(cmd_iter)),
                    "XREAD" => Self::ImmeCommand(ImmeCommand::Xread(cmd_iter)),
                    "INCR" => Self::ImmeCommand(ImmeCommand::Incr(cmd_iter)),
                    "WATCH" => Self::TransCommand(TransCommand::Watch(cmd_iter)),
                    "UNWATCH" => Self::TransCommand(TransCommand::Unwatch),
                    "MULTI" => Self::TransCommand(TransCommand::Multi),
                    "EXEC" => Self::TransCommand(TransCommand::Exec),
                    "DISCARD" => Self::TransCommand(TransCommand::Discard),
                    "REPLCONF" => Self::ImmeCommand(ImmeCommand::Replconf(cmd_iter)),
                    "PSYNC" => Self::ImmeCommand(ImmeCommand::Psync(cmd_iter)),
                    _ => bail!("unsupported command!"),
                };
                return Ok(cmd);
            }
            _ => {
                bail!("command format error!");
            }
        }
    }

    pub fn execute(self, ctx: &Arc<Context>, trans: &mut Transaction) -> Result<CommandResponse> {
        match self {
            Self::ImmeCommand(imme_cmd) => imme_cmd.execute(ctx, trans),
            Self::TransCommand(trans_cmd) => trans_cmd.execute(ctx, trans),
        }
    }
}
