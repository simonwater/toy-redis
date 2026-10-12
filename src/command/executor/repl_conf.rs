use crate::{Command, CommandResponse, Context, Value};
use anyhow::Result;
use std::sync::Arc;

pub(super) fn execute_replconf(_cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
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
}
