use crate::user::UserToken;
use crate::{Command, CommandResponse, Context, Session, Value};
use anyhow::{Result, anyhow, bail};
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
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let users_hub = ctx.users_hub();
    let cur_user = users_hub.current_user(session)?;
    Ok(cur_user.name().into())
}

fn execute_getuser(
    mut _args_iter: Iter<Value>,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let users_hub = ctx.users_hub();
    let cur_user = users_hub.current_user(session)?;
    let user_value = cur_user.to_value();
    Ok(user_value.into())
}

fn execute_setuser(
    mut args_iter: Iter<Value>,
    ctx: &Arc<Context>,
    session: &mut Session,
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
    let user = users_hub.get_user_by_name(&name).ok_or_else(|| {
        anyhow!(
            "ERR acl setuser command error, user: {:?} is not exist",
            name
        )
    })?;

    match &rule.as_bytes()[0] {
        b'>' => {
            let pwd_bytes = user.set_password(&rule[1..])?;
            if session.current_user_token().is_none() {
                // 设置密码时如果当前用户没认证，直接认证为当前用户
                let token = UserToken::new(name, pwd_bytes);
                users_hub.auth_user_token(user, token, session);
            }
            Ok("OK".into())
        }
        _ => bail!("ERR acl setuser command does not support {} rule", rule),
    }
}

pub(super) fn execute_auth(
    cmd: &Command,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let name = args_iter
        .next()
        .ok_or_else(|| anyhow!("ERR auth command missing user name argument!"))?
        .to_bulk_bytes()?;
    let password = args_iter
        .next()
        .ok_or_else(|| anyhow!("ERR auth command missing password argument!"))?
        .to_string()?;
    let users_hub = ctx.users_hub();
    users_hub.auth(&name, &password, session)?;
    Ok("OK".into())
}
