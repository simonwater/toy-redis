use crate::{
    AofMiddleware, CommandPipeline, CommandResponse, ConnectionHandler, ConnectionState, Context,
    ReplConfSlaveMiddleware, Session, TransMiddleware, Value,
};
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::Arc;
use std::thread;

pub fn start(ctx: &Arc<Context>) -> Result<()> {
    // 从库方式启动
    let args = ctx.args_ref();
    if let Some(addr) = args.replicaof.as_deref() {
        let mut conn = ConnectionHandler::new(addr.replace(" ", ":"))?;

        // ping
        let cmd = Value::BulkStrings("PING".into());
        let (value, _) = conn.request(cmd)?.unwrap();
        assert_eq!(value, Value::SimpleStrings("PONG".into()));

        // replconf 1
        let cmd = vec![
            Value::BulkStrings("REPLCONF".into()),
            Value::BulkStrings("listening-port".into()),
            Value::BulkStrings(Bytes::from(args.port.clone())),
        ];
        let (value, _) = conn.request(Value::Arrays(cmd))?.unwrap();
        assert_eq!(value, Value::SimpleStrings("OK".into()));

        // replconf 2
        let cmd = vec![
            Value::BulkStrings("REPLCONF".into()),
            Value::BulkStrings("capa".into()),
            Value::BulkStrings("psync2".into()),
        ];
        let (value, _) = conn.request(Value::Arrays(cmd))?.unwrap();
        assert_eq!(value, Value::SimpleStrings("OK".into()));

        // psync
        let cmd = vec![
            Value::BulkStrings("PSYNC".into()),
            Value::BulkStrings("?".into()),
            Value::BulkStrings("-1".into()),
        ];
        let _init_msg = conn.send(Value::Arrays(cmd))?;
        let _init_value = conn.receive_value()?;
        let _rdb_stream = conn.receive_value()?;
        let ctx = ctx.clone();
        thread::spawn(move || {
            let mut repl_pipeline = repl_pipeline();
            conn.reset_cmd_total_bytes();
            let mut session = Session::new().with_transaction().with_connection(conn);
            // 增量命令
            loop {
                match handle_repl_command(&mut repl_pipeline, &ctx, session) {
                    Ok(keep_session) => session = keep_session,
                    Err(err) => {
                        eprintln!("Replica synchronization err: {:?}", err);
                        return;
                    }
                }
            }
        });
    }
    Ok(())
}

fn handle_repl_command(
    pipeline: &mut CommandPipeline,
    ctx: &Arc<Context>,
    mut session: Session,
) -> Result<Session> {
    let conn = session.connection_mut()?;
    if let Some(command) = conn.receive_command_with_stat()? {
        let response = pipeline.execute(&command, &ctx, &mut session)?;
        if matches!(response, CommandResponse::ReplConf(_)) {
            match response.run(&ctx, session)? {
                // ReplConf类型的Response需要响应结果
                ConnectionState::KeepAlive(keep_session) => return Ok(keep_session),
                _ => bail!("unsupported connection state!"),
            }
        }
        // 其他类型Response不返回结果
        Ok(session)
    } else {
        bail!("master server is closed!")
    }
}

/// 主从同步的副本执行命令时用到的中间件： 事务、replconf
fn repl_pipeline() -> CommandPipeline {
    let pipeline = CommandPipeline::new();
    pipeline
        .use_middleware(Box::new(TransMiddleware)) // 事务控制中间件
        .use_middleware(Box::new(ReplConfSlaveMiddleware)) // REPLCONF 命令处理中间件
        .use_middleware(Box::new(AofMiddleware)) //aof存储中间件
}
