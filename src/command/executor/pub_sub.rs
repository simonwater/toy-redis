use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;

use crate::{Command, CommandResponse, Context, Session, Value};

pub(super) fn execute_subscribe(
    cmd: &Command,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let chennal = arg_iter
        .next()
        .ok_or_else(|| anyhow!("subscribe command missing chennal argument!"))?
        .to_bulk_bytes()?;
    let subs = session.subscription_mut()?;
    subs.add_chennal(chennal.clone(), ctx);
    let cnt = subs.len();
    let result = vec![
        Bytes::from_static(b"subscribe").into(),
        chennal.into(),
        Value::Integer(cnt as i64),
    ];
    Ok(result.into())
}

pub(super) fn execute_publish(
    cmd: &Command,
    ctx: &Arc<Context>,
    _session: &mut Session,
) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let chennal = arg_iter
        .next()
        .ok_or_else(|| anyhow!("subscribe command missing chennal argument!"))?
        .to_bulk_bytes()?;

    let _content = arg_iter
        .next()
        .ok_or_else(|| anyhow!("subscribe command missing content argument!"))?
        .to_bulk_bytes()?;

    let cnt = ctx.subcriber_hub().client_cnt(chennal) as i64;
    Ok(cnt.into())
}
