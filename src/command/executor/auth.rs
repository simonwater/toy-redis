use crate::{Command, CommandResponse, Context, Session};
use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use std::sync::Arc;

pub(super) fn execute_acl(
    cmd: &Command,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let arg1 = arg_iter
        .next()
        .ok_or_else(|| anyhow!("ERR acl command need one argument at least!"))?
        .to_string()?;
    match arg1.to_uppercase().as_str() {
        "WHOAMI" => execute_whoami(cmd, ctx, session),
        "GETUSER" => execute_getuser(cmd, ctx, session),
        _ => bail!("ERR acl command does not support {} argument", arg1),
    }
}

fn execute_whoami(
    _cmd: &Command,
    _ctx: &Arc<Context>,
    _session: &mut Session,
) -> Result<CommandResponse> {
    Ok(Bytes::from("default").into())
}

fn execute_getuser(
    _cmd: &Command,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let users_hub = ctx.users_hub();
    let cur_user = users_hub.current_user(session);
    let user_value = cur_user.to_value();
    Ok(user_value.into())
}
