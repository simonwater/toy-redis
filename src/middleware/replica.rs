use super::Middleware;
use crate::{Command, CommandResponse, Context, Transaction};
use anyhow::Result;
use std::sync::Arc;

pub struct ReplicaMiddleware;

impl Middleware for ReplicaMiddleware {
    fn handle(
        &self,
        cmd: &Command,
        ctx: &Arc<Context>,
        trans: &mut Transaction,
        next: &dyn Fn(&mut Transaction) -> Result<CommandResponse>,
    ) -> Result<CommandResponse> {
        let result = next(trans);

        if ctx.is_master() && !ctx.repl_hub().is_empty() {
            match cmd.name.as_str() {
                "SET" | "INCR" | "RPUSH" | "LPUSH" | "RPOP" | "LPOP" | "BRPOP" | "BLPOP"
                | "XADD" => ctx.repl_hub().prop_command(cmd),
                _ => {}
            }
        }

        result
    }
}
