use crate::{Command, CommandResponse, Context};
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub(super) fn execute_setbit(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let key = arg_iter
        .next()
        .ok_or_else(|| anyhow!("setbit command missing bit map key!"))?
        .to_bulk_bytes()?;
    let offset: usize = arg_iter
        .next()
        .ok_or_else(|| anyhow!("setbit command missing offset argument!"))?
        .to_integer()?
        .try_into()?;
    let bit_val: u8 = arg_iter
        .next()
        .ok_or_else(|| anyhow!("setbit command missing bit value argument!"))?
        .to_integer()?
        .try_into()?;

    let db = ctx.db_ref();
    let old_val = db.set_bit(key, offset, bit_val)? as i64;
    Ok(old_val.into())
}
