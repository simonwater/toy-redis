use super::Middleware;
use crate::{Command, CommandResponse, Context, Session};
use anyhow::Result;
use std::sync::Arc;

pub struct TransMiddleware;

impl Middleware for TransMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        let trans = session.trans_mut();
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

        next(session)
    }
}
