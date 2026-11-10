use super::Middleware;
use crate::{Command, CommandResponse, Context, Session};
use anyhow::Result;
use std::sync::Arc;

pub struct AuthMiddleware;

impl Middleware for AuthMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        if cmd.name.as_str() != "AUTH" {
            let users_hub = ctx.users_hub();
            users_hub.current_user(session)?;
        }
        next(session)
    }
}
