use crate::{Command, CommandResponse, Context};
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub(super) fn execute_zadd(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let set_key = cmd
        .args
        .iter()
        .next()
        .ok_or_else(|| anyhow!("zadd command missing set key!"))?
        .to_bulk_bytes()?;

    let chunks = cmd.args[1..].chunks_exact(2);
    let mut args = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let score = chunk[0].to_double()?;
        let member = chunk[1].to_bulk_bytes()?;
        args.push((score, member));
    }

    let db = ctx.db_ref();
    let len = db.zadd(set_key, args)?;
    Ok(len.into())
}
