mod repl_conf;
mod replica;
mod trans;

use crate::{CmdExecutor, Command, CommandResponse, Context, context::Session};
use anyhow::Result;
use std::sync::Arc;

pub use repl_conf::ReplConfMiddleware;
pub use replica::ReplicaMiddleware;
pub use trans::TransMiddleware;

pub trait Middleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        session: &mut Session,
        next: &dyn Fn(&mut Session) -> Result<CommandResponse>,
    ) -> Result<CommandResponse>;
}

pub struct CommandPipeline {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl CommandPipeline {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::with_capacity(16),
        }
    }

    pub fn use_middleware(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    pub fn execute(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        session: &mut Session,
    ) -> Result<CommandResponse> {
        self.execute_at(cmd, 0, ctx, session)
    }

    fn execute_at(
        &self,
        cmd: &Command,
        index: usize,
        ctx: &Arc<Context>,
        session: &mut Session,
    ) -> Result<CommandResponse> {
        if index < self.middlewares.len() {
            let mw = &self.middlewares[index];
            mw.handle(cmd, ctx, session, &|session| {
                self.execute_at(cmd, index + 1, ctx, session)
            })
        } else {
            CmdExecutor::execute(cmd, ctx)
        }
    }
}
