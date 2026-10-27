use super::Middleware;
use crate::{Command, CommandResponse, Context, Session};
use anyhow::Result;
use std::sync::Arc;

pub struct AofMiddleware;

impl Middleware for AofMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        let result = next(session);
        ctx.aof_handler().append_command(cmd)?;
        result
    }
}
