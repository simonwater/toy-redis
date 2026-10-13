use super::Middleware;
use crate::{Command, CommandResponse, Context, Session};
use anyhow::Result;
use std::sync::Arc;

pub struct ReplicaMiddleware;

impl Middleware for ReplicaMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        let result = next(session);

        if ctx.is_master() && !ctx.repl_hub().is_empty() {
            match cmd.name.as_str() {
                "SET" | "INCR" | "RPUSH" | "LPUSH" | "RPOP" | "LPOP" | "BRPOP" | "BLPOP"
                | "XADD" | "WATCH" | "UNWATCH" | "MULTI" | "EXEC" | "DISCARD" => {
                    ctx.repl_hub().prop_command(cmd)
                }
                _ => {}
            }
        }

        result
    }
}
