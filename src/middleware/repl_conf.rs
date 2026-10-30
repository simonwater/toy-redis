use super::Middleware;
use crate::{Command, CommandResponse, Context, Session, Value};
use anyhow::{Result, bail};
use std::sync::Arc;

pub struct ReplConfMasterMiddleware;

impl Middleware for ReplConfMasterMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        _ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        if cmd.name.as_str() == "REPLCONF" {
            return Ok("OK".into());
        }
        return next(session);
    }
}

/// 从库REPLCONF命令处理中间件
pub struct ReplConfSlaveMiddleware;

impl Middleware for ReplConfSlaveMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        _ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        if cmd.name.as_str() != "REPLCONF" {
            return next(session);
        }

        if let Some(value) = cmd.args.iter().next() {
            let arg1 = value.to_string()?;
            match arg1.as_str() {
                "GETACK" => {
                    let total_cmd_bytes = session.connection_ref()?.get_cmd_total_bytes();
                    let ack_res = total_cmd_bytes - cmd.bytes.len();
                    let values: Vec<Value> = vec![
                        Value::BulkStrings("REPLCONF".into()),
                        Value::BulkStrings("ACK".into()),
                        Value::BulkStrings(ack_res.to_string().into()),
                    ];
                    return Ok(CommandResponse::ReplConf(values.into()));
                }
                arg => bail!("err: unsupported argument {} for REPLCONF command", arg),
            }
        } else {
            bail!("err: REPLCONF command missing arguments")
        }
    }
}
