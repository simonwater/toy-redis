use crate::{Command, CommandResponse, Context, Transaction, Value};
use std::sync::Arc;

pub trait Middleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        trans: &mut Transaction,
        next: &dyn Fn(&mut Transaction) -> CommandResponse,
    ) -> CommandResponse;
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
    ) -> CommandResponse {
        self.execute_at(cmd, 0, ctx, trans)
    }

    fn execute_at(
        &self,
        cmd: &Command,
        index: usize,
        ctx: &Arc<Context>,
        trans: &mut Transaction,
    ) -> CommandResponse {
        if index < self.middlewares.len() {
            let mw = &self.middlewares[index];
            mw.handle(cmd, ctx, trans, &|trans| {
                self.execute_at(cmd, index + 1, ctx, trans)
            })
        } else {
            // let response = match cmd.execute(ctx, trans) {
            //     Ok(response) => response,
            //     Err(e) => Value::SimpleErrors(format!("{}", e)).into(),
            // };

            //response
            "todo".into()
        }
    }
}
