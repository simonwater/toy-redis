use anyhow::{Error, Result, anyhow};
use bytes::Bytes;
use std::{
    io::{self, ErrorKind},
    sync::Arc,
    time::Duration,
};

use crate::{Command, CommandResponse, ConnectionState, Context, Session, Value};

pub(super) fn execute_subscribe(
    cmd: &Command,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    // 进入订阅模式
    let val = handle_subscribe(cmd, ctx, session)?;
    session.write_value(val)?;
    let result = CommandResponse::TakeOver(Box::new(move |ctx, mut session| {
        subscribe_loop(ctx, &mut session)?;
        Ok(ConnectionState::Close)
    }));
    Ok(result.into())
}

fn subscribe_loop(ctx: &Arc<Context>, session: &mut Session) -> Result<()> {
    let conn = session.connection_mut()?;
    conn.get_stream()
        .set_read_timeout(Some(Duration::from_millis(100)))?;

    // 约定循环内部代码返回异常时（?）退出循环
    loop {
        // 接收channel订阅消息
        try_recv_channelmsg(session)?;

        // 接收客户端命令，等待时间100ms
        match session.connection_mut()?.receive_command() {
            Ok(Some(command)) => {
                // 执行命令
                if command.name.as_str() == "QUIT" {
                    session.write_value(Value::SimpleStrings("OK".into()))?;
                    return Ok(());
                }

                let cmd_result = match handle_loop_cmd(&command, ctx, session) {
                    Ok(value) => value,
                    Err(e) => {
                        eprintln!("subscribe mode command error: {}", e);
                        Value::SimpleErrors(format!("{}", e))
                    }
                };
                session.write_value(cmd_result)?;
            }
            Ok(None) => return Ok(()), // 客户端关闭
            Err(err) => {
                if is_timeout(&err) {
                    continue;
                } else {
                    return Err(err);
                }
            }
        }
    }
}

fn is_timeout(err: &Error) -> bool {
    if let Some(io_err) = err.downcast_ref::<io::Error>()
        && (io_err.kind() == ErrorKind::TimedOut || io_err.kind() == ErrorKind::WouldBlock)
    {
        return true;
    }
    false
}

fn try_recv_channelmsg(session: &mut Session) -> Result<()> {
    // 接收channel订阅消息
    let channel_msgs = session.subscription_ref()?.try_recv_msgs();
    for channel_msg in channel_msgs {
        let val = Value::Arrays(vec![
            Bytes::from_static(b"message").into(),
            channel_msg.channel.into(),
            channel_msg.message.into(),
        ]);
        session.connection_mut()?.write_all(&val.to_bytes())?;
    }
    Ok(())
}

fn handle_loop_cmd(command: &Command, ctx: &Arc<Context>, session: &mut Session) -> Result<Value> {
    let cmd_result = match command.name.as_str() {
        "SUBSCRIBE" => handle_subscribe(&command, ctx, session)?,
        "UNSUBSCRIBE" => handle_unsubscribe(&command, ctx, session)?,
        "PSUBSCRIBE" => Value::SimpleErrors("unsupported command: PSUBSCRIBE".into()),
        "PUNSUBSCRIBE" => Value::SimpleErrors("unsupported command: PUNSUBSCRIBE".into()),
        "PING" => handle_ping()?,
        _ => Value::SimpleErrors(format!(
            "ERR Can't execute '{}': only (P|S)SUBSCRIBE / (P|S)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context",
            command.name,
        )),
    };
    Ok(cmd_result)
}

fn handle_subscribe(cmd: &Command, ctx: &Arc<Context>, session: &mut Session) -> Result<Value> {
    let mut arg_iter = cmd.args.iter();
    let channel = arg_iter
        .next()
        .ok_or_else(|| anyhow!("subscribe command missing channel argument!"))?
        .to_bulk_bytes()?;
    let sub = session.subscription_mut()?;
    let len = sub.subscribe(channel.clone(), ctx);
    let val = Value::Arrays(vec![
        Bytes::from_static(b"subscribe").into(),
        channel.into(),
        Value::Integer(len as i64),
    ]);

    Ok(val)
}

fn handle_unsubscribe(cmd: &Command, ctx: &Arc<Context>, session: &mut Session) -> Result<Value> {
    let mut arg_iter = cmd.args.iter();
    let channel = arg_iter
        .next()
        .ok_or_else(|| anyhow!("unsubscribe command missing channel argument!"))?
        .to_bulk_bytes()?;
    let sub = session.subscription_mut()?;
    let len = sub.unsubscribe(channel.clone(), ctx);
    let val = Value::Arrays(vec![
        Bytes::from_static(b"unsubscribe").into(),
        channel.into(),
        Value::Integer(len as i64),
    ]);

    Ok(val)
}

fn handle_ping() -> Result<Value> {
    let val: Value = Value::Arrays(vec![
        Bytes::from_static(b"pong").into(),
        Bytes::from_static(b"").into(),
    ]);
    Ok(val)
}

pub(super) fn execute_publish(
    cmd: &Command,
    ctx: &Arc<Context>,
    _session: &mut Session,
) -> Result<CommandResponse> {
    let mut arg_iter = cmd.args.iter();
    let channel = arg_iter
        .next()
        .ok_or_else(|| anyhow!("publish command missing channel argument!"))?
        .to_bulk_bytes()?;

    let msg = arg_iter
        .next()
        .ok_or_else(|| anyhow!("publish command missing content argument!"))?
        .to_bulk_bytes()?;

    let cnt = ctx.channels_hub().publish(channel, msg) as i64;
    Ok(cnt.into())
}
