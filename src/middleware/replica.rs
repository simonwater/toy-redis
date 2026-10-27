use super::Middleware;
use crate::{Command, CommandResponse, Context, ReplMsg, Session};
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

        if ctx.is_master() && !ctx.repl_hub().is_empty() && cmd.is_write() {
            let msg = ReplMsg::Propagation(cmd.bytes.clone());
            ctx.repl_hub().prop_replmsg(msg)
        }

        result
    }
}
