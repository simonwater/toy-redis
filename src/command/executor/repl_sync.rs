use super::CommandResponse;
use crate::{Command, ConnectionState, Context, ReplMsg, Value};
use anyhow::{Result, anyhow, bail};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

pub(super) fn execute_psync(_cmd: &Command, _ctx: &Arc<Context>) -> Result<CommandResponse> {
    // 接管后续网络数据回写流程，消耗session，用于构建起和副本的连接通道
    let response = CommandResponse::TakeOver(Box::new(|ctx, mut session| {
        let conn = session.connection_mut()?;
        let master_id = ctx.info_ref().get_master_replid();
        let val = Value::SimpleStrings(format!("FULLRESYNC {} 0", master_id));
        let rdb_vec = empty_rdb();
        let len = rdb_vec.len();
        let mut stream = Cursor::new(rdb_vec);
        conn.write_all(&val.to_bytes())?;
        let head = format!("${}\r\n", len);
        conn.write_all(head.as_bytes())?;
        std::io::copy(&mut stream, conn.get_stream())?;
        ctx.repl_hub().register_repla(session);
        Ok(ConnectionState::Close)
    }));
    Ok(response)
}

fn empty_rdb() -> Vec<u8> {
    const EMPTY_RDB_HEX: &'static str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";
    (0..EMPTY_RDB_HEX.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&EMPTY_RDB_HEX[i..i + 2], 16).unwrap())
        .collect()
}

pub(super) fn execute_wait(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let repl_hub = ctx.repl_hub();
    let repl_len = repl_hub.len() as i64;
    if repl_hub.repl_command_count() == 0 {
        return Ok(repl_len.into());
    }

    let mut arg_iter = cmd.args.iter();
    let numreplicas = arg_iter
        .next()
        .ok_or_else(|| anyhow!("wait command missing `numreplicas` argument"))?
        .to_integer()?;
    let timeout = arg_iter
        .next()
        .ok_or_else(|| anyhow!("wait command missing `timeout` argument"))?
        .to_integer()?;
    let total_time = Duration::from_millis(timeout as u64);

    let (sender, receiver) = mpsc::channel();
    let sender = Arc::new(sender);
    let msg = ReplMsg::GetAck {
        respond: sender.clone(),
        timeout: total_time,
    };
    repl_hub.prop_replmsg(msg);

    let numreplicas = numreplicas.min(repl_len);
    let mut res = 0;
    let start = Instant::now();
    loop {
        if res == numreplicas {
            break;
        }
        let pasted_time = Instant::now() - start;
        if pasted_time >= total_time {
            break;
        }
        let remaining_time = total_time - pasted_time;
        match receiver.recv_timeout(remaining_time) {
            Ok(_offset) => res += 1,
            Err(e) => match e {
                RecvTimeoutError::Timeout => {
                    eprintln!("time out when wait receive respond");
                    break;
                }
                RecvTimeoutError::Disconnected => {
                    bail!("error when wait receive respond: {}", e)
                }
            },
        }
    }

    Ok(res.into())
}
