mod basic;
mod list;
mod stream;

use super::{Command, Transaction, response::CommandResponse};
use crate::Context;
use anyhow::{Result, bail};
use std::sync::Arc;

pub fn execute(
    cmd: &Command,
    ctx: &Arc<Context>,
    trans: &mut Transaction,
) -> Result<CommandResponse> {
    match cmd.name.as_str() {
        "WATCH" => return trans.watch(cmd, ctx),
        "UNWATCH" => return trans.unwatch(),
        "MULTI" => return trans.start(),
        "EXEC" => return trans.exec(ctx),
        "DISCARD" => return trans.discard(),
        _ => {}
    };

    if trans.is_started() {
        let res = trans.add_command(cmd.clone())?; // 延迟统一处理
        return Ok(res);
    }
    execute_basic(cmd, ctx)
}

pub(super) fn execute_basic(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    match cmd.name.as_str() {
        "PING" => basic::execute_ping(cmd, ctx),
        "ECHO" => basic::execute_echo(cmd, ctx),
        "INFO" => basic::execute_info(cmd, ctx),
        "GET" => basic::execute_get(cmd, ctx),
        "SET" => basic::execute_set(cmd, ctx),
        "TYPE" => basic::execute_type(cmd, ctx),
        "INCR" => basic::execute_incr(cmd, ctx),
        "RPUSH" => list::execute_rpush(cmd, ctx),
        "LPUSH" => list::execute_lpush(cmd, ctx),
        "RPOP" => list::execute_rpop(cmd, ctx),
        "LPOP" => list::execute_lpop(cmd, ctx),
        "BRPOP" => list::execute_brpop(cmd, ctx),
        "BLPOP" => list::execute_blpop(cmd, ctx),
        "LRANGE" => list::execute_lrange(cmd, ctx),
        "LLEN" => list::execute_llen(cmd, ctx),
        "XADD" => stream::execute_xadd(cmd, ctx),
        "XRANGE" => stream::execute_xrange(cmd, ctx),
        "XREAD" => stream::execute_xread(cmd, ctx),
        "REPLCONF" => basic::execute_replconf(cmd, ctx),
        "PSYNC" => basic::execute_psync(cmd, ctx),
        _ => bail!("unsupported command!"),
    }
}
