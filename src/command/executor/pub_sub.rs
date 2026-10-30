use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;

use crate::{Command, CommandResponse, Context, Session, Value};

pub(super) fn execute_subscribe(
    cmd: &Command,
    _ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let chennal = arg_iter
        .next()
        .ok_or_else(|| anyhow!("subscribe command missing chennal argument!"))?
        .to_bulk_bytes()?;
    session.add_chennal(chennal.clone());
    let cnt = session.chennal_count();
    let result = vec![
        Bytes::from_static(b"subscribe").into(),
        chennal.into(),
        Value::Integer(cnt as i64),
    ];
    Ok(result.into())
}
