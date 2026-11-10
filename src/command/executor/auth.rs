use crate::{Command, CommandResponse, Context, Session, Value};
use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use std::slice::Iter;
use std::sync::Arc;

pub(super) fn execute_acl(
    cmd: &Command,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let arg1 = args_iter
        .next()
        .ok_or_else(|| anyhow!("ERR acl command need one argument at least!"))?
        .to_string()?;
    match arg1.to_uppercase().as_str() {
        "WHOAMI" => execute_whoami(args_iter, ctx, session),
        "GETUSER" => execute_getuser(args_iter, ctx, session),
        "SETUSER" => execute_setuser(args_iter, ctx, session),
        _ => bail!("ERR acl command does not support {} argument", arg1),
    }
}

fn execute_whoami(
    mut _args_iter: Iter<Value>,
    _ctx: &Arc<Context>,
    _session: &mut Session,
) -> Result<CommandResponse> {
    Ok(Bytes::from("default").into())
}

fn execute_getuser(
    mut _args_iter: Iter<Value>,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let users_hub = ctx.users_hub();
    let cur_user = users_hub.current_user(session);
    let user_value = cur_user.to_value();
    Ok(user_value.into())
}

fn execute_setuser(
    mut args_iter: Iter<Value>,
    ctx: &Arc<Context>,
    _session: &mut Session,
) -> Result<CommandResponse> {
    let name = args_iter
        .next()
        .ok_or_else(|| anyhow!("ERR acl setuser command missing user name argument!"))?
        .to_bulk_bytes()?;
    let rule = args_iter
        .next()
        .ok_or_else(|| anyhow!("ERR acl setuser command missing rule argument!"))?
        .to_string()?;

    let users_hub = ctx.users_hub();
    let user = users_hub.get_user(&name).ok_or_else(|| {
        anyhow!(
            "ERR acl setuser command error, user: {:?} is not exist",
            name
        )
    })?;

    match &rule.as_bytes()[0] {
        b'>' => {
            user.set_password(&rule[1..]);
            Ok("OK".into())
        }
        _ => bail!("ERR acl setuser command does not support {} rule", rule),
    }
}
