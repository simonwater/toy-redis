use crate::{Command, CommandResponse, Context};
use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use std::sync::Arc;

pub(super) fn execute_acl(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let arg1 = arg_iter
        .next()
        .ok_or_else(|| anyhow!("ERR acl command need one argument at least!"))?
        .to_string()?;
    match arg1.to_uppercase().as_str() {
        "WHOAMI" => execute_whoami(cmd, ctx),
        _ => bail!("ERR acl command is not support {} argument", arg1),
    }
}

fn execute_whoami(_cmd: &Command, _ctx: &Arc<Context>) -> Result<CommandResponse> {
    Ok(Bytes::from("default").into())
}
