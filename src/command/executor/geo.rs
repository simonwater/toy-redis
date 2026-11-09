use crate::{Command, CommandResponse, Context};
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub(super) fn execute_geoadd(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let key = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing key!"))?
        .to_bulk_bytes()?;
    let longitude = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing longitude argument!"))?
        .to_double()?;
    let latitude = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing latitude argument!"))?
        .to_double()?;
    let member = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing member argument!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let cnt = db.geoadd(key, longitude, latitude, member)?;
    Ok(cnt.into())
}
