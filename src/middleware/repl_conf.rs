use super::Middleware;
use crate::{Command, CommandResponse, Context, Session, Value};
use anyhow::Result;
use std::sync::Arc;

pub struct ReplConfMiddleware;

impl Middleware for ReplConfMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        if cmd.name.as_str() == "REPLCONF" {
            if ctx.is_master() {
                // 主服务
                return Ok("OK".into());
            } else {
                // 复本
                let values: Vec<Value> = vec![
                    Value::BulkStrings("REPLCONF".into()),
                    Value::BulkStrings("ACK".into()),
                    Value::BulkStrings("0".into()),
                ];
                return Ok(CommandResponse::ReplConf(values.into()));
            }
        } else {
            next(session)
        }
    }
}
