use super::CommandResponse;
use crate::{Command, Context, ReplMsg};
use anyhow::{Result, anyhow, bail};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

pub(super) fn execute_psync(_cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let master_id = ctx.info_ref().get_master_replid();

    let s: String = format!("FULLRESYNC {} 0", master_id);
    let stream = empty_rdb();
    let len = stream.len();
    let response = CommandResponse::Replication {
        init_value: s.into(),
        stream: Box::new(Cursor::new(stream)),
        len,
    };
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
    ctx.repl_hub().prop_replmsg(msg);

    let numreplicas = numreplicas.min(ctx.repl_hub().len() as i64);
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
