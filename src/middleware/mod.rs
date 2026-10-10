mod trans;

use crate::{CmdExecutor, Command, CommandResponse, Context, Transaction};
use anyhow::Result;
use std::sync::Arc;

pub use trans::TransMiddleware;

pub trait Middleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        trans: &mut Transaction,
        next: &dyn Fn(&mut Transaction) -> Result<CommandResponse>,
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
        trans: &mut Transaction,
    ) -> Result<CommandResponse> {
        self.execute_at(cmd, 0, ctx, trans)
    }

    fn execute_at(
        &self,
        cmd: &Command,
        index: usize,
        ctx: &Arc<Context>,
        trans: &mut Transaction,
    ) -> Result<CommandResponse> {
        if index < self.middlewares.len() {
            let mw = &self.middlewares[index];
            mw.handle(cmd, ctx, trans, &|trans| {
                self.execute_at(cmd, index + 1, ctx, trans)
            })
        } else {
            CmdExecutor::execute(cmd, ctx)
        }
    }
}
