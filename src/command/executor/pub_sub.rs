use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use std::{
    sync::{Arc, mpsc::Receiver},
    time::Duration,
};

use crate::{Command, CommandResponse, ConnectionState, Context, Session, Value};

pub(super) fn execute_subscribe(
    cmd: &Command,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<CommandResponse> {
    let mut rx_list = Vec::with_capacity(16);
    let val = handle_subscribe(cmd, &mut rx_list, ctx, session)?;
    session.connection_mut()?.write_all(&val.to_bytes())?;
    let result = CommandResponse::TakeOver(Box::new(move |ctx, mut session| {
        if let Err(e) = start_loop(ctx, &mut session, rx_list) {
            eprintln!("{}", e);
            let val = Value::SimpleErrors(format!("{}", e));
            session.connection_mut()?.write_all(&val.to_bytes())?;
        }
        Ok(ConnectionState::Close)
    }));
    Ok(result.into())
}

fn start_loop(
    ctx: &Arc<Context>,
    session: &mut Session,
    mut rx_list: Vec<(Bytes, Receiver<Bytes>)>,
) -> Result<()> {
    let conn = session.connection_mut()?;
    conn.get_stream()
        .set_read_timeout(Some(Duration::from_millis(100)))?;
    loop {
        // 接收chennal订阅消息
        try_recv_msg(session, &rx_list)?;

        // 接收客户端命令
        match session.connection_mut()?.receive_command() {
            Ok(Some(command)) => {
                // 执行命令
                let cmd_result = match command.name.as_str() {
                    "QUIT" => {
                        session
                            .connection_mut()?
                            .write_all(&Value::SimpleStrings("OK".into()).to_bytes())?;

                        return Ok(());
                    }
                    _ => exec_cmd(&command, &mut rx_list, ctx, session)?,
                };
                session
                    .connection_mut()?
                    .write_all(&cmd_result.to_bytes())?;
            }
            Ok(None) => return Ok(()),
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::TimedOut
                        || io_err.kind() == std::io::ErrorKind::WouldBlock
                    {
                        // 超时处理
                        continue;
                    }
                } else {
                    bail!(e)
                }
            }
        }
    }
}

fn try_recv_msg(session: &mut Session, rx_list: &Vec<(Bytes, Receiver<Bytes>)>) -> Result<()> {
    // 接收chennal订阅消息
    for (chennal, rx) in rx_list {
        if let Ok(msg) = rx.try_recv() {
            let val = Value::Arrays(vec![
                Bytes::from_static(b"message").into(),
                chennal.clone().into(),
                msg.into(),
            ]);
            session.connection_mut()?.write_all(&val.to_bytes())?;
        }
    }
    Ok(())
}

fn exec_cmd(
    command: &Command,
    rx_list: &mut Vec<(Bytes, Receiver<Bytes>)>,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<Value> {
    let cmd_result = match command.name.as_str() {
        "SUBSCRIBE" => handle_subscribe(&command, rx_list, ctx, session)?,
        "UNSUBSCRIBE" => Value::SimpleErrors("unsupported command: UNSUBSCRIBE".into()),
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

fn handle_subscribe(
    cmd: &Command,
    rx_list: &mut Vec<(Bytes, Receiver<Bytes>)>,
    ctx: &Arc<Context>,
    session: &mut Session,
) -> Result<Value> {
    let mut arg_iter = cmd.args.iter();
    let chennal = arg_iter
        .next()
        .ok_or_else(|| anyhow!("subscribe command missing chennal argument!"))?
        .to_bulk_bytes()?;
    let sub = session.subscription_mut()?;
    if let Some(rx) = sub.subscirbe(chennal.clone(), ctx) {
        rx_list.push((chennal.clone(), rx));
    }
    let val = Value::Arrays(vec![
        Bytes::from_static(b"subscribe").into(),
        chennal.into(),
        Value::Integer(sub.len() as i64),
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
    let chennal = arg_iter
        .next()
        .ok_or_else(|| anyhow!("publish command missing chennal argument!"))?
        .to_bulk_bytes()?;

    let msg = arg_iter
        .next()
        .ok_or_else(|| anyhow!("publish command missing content argument!"))?
        .to_bulk_bytes()?;

    let cnt = ctx.chennals_hub().publish(chennal, msg) as i64;
    Ok(cnt.into())
}
