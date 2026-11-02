use super::Middleware;
use crate::{Command, CommandResponse, Context, Session};
use anyhow::{Result, bail};
use std::sync::Arc;

/// subscribed 模式控制中间件
pub struct SubscribeMiddleware;

impl Middleware for SubscribeMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        _ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        if !session.is_subscribe_mode() || match_sub_command(&cmd.name) {
            next(session)
        } else {
            bail!(
                "ERR Can't execute '{}': only (P|S)SUBSCRIBE / (P|S)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context",
                cmd.name
            )
        }
    }
}

fn match_sub_command(name: &str) -> bool {
    match name {
        "SUBSCRIBE" | "UNSUBSCRIBE" | "PSUBSCRIBE" | "PUNSUBSCRIBE" | "PING" | "QUIT" => true,
        _ => false,
    }
}
