use anyhow::{Result, bail};
use bytes::Bytes;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use toy_redis::{
    AofMiddleware, CommandPipeline, CommandResponse, ConnectionHandler, ConnectionState, Context,
    ReplConfMasterMiddleware, ReplConfSlaveMiddleware, ReplicaMiddleware, Session, TransMiddleware,
    Value,
};

fn main() {
    let ctx_arc = Arc::new(Context::init().expect("redis context init error"));
    let port = &ctx_arc.args_ref().port;
    println!("Server is started on port: {}!", port);

    // 主从同步副本启动逻辑
    handle_repl(&ctx_arc).unwrap();
    // aof重放
    let aof_handler = ctx_arc.aof_handler();
    aof_handler.replay(&ctx_arc).expect("aof replay error");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    for stream in listener.incoming() {
        let ctx = ctx_arc.clone();
        println!("accepted new connection");
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    let conn = ConnectionHandler::from_tcp_stream(stream);
                    let session = Session::new(conn);
                    if let Err(e) = handle_connection(ctx, session) {
                        eprintln!("err: {:?}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("err: {:?}", e);
            }
        }
    }
}

fn handle_connection(ctx: Arc<Context>, mut session: Session) -> Result<()> {
    let cmd_pipeline = get_command_pipeline();
    // 读取命令
    while let Some(command) = session.connection_mut().receive_command()? {
        // 执行命令
        let response = match cmd_pipeline.execute(&command, &ctx, &mut session) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("command pipeline execute error: {}", e);
                Value::SimpleErrors(format!("{}", e)).into()
            }
        };

        // 回写响应
        match response.run(&ctx, session)? {
            ConnectionState::KeepAlive(keep_session) => session = keep_session,
            ConnectionState::TakenOver => break,
        }
    }
    Ok(())
}

fn get_command_pipeline() -> CommandPipeline {
    let pipeline = CommandPipeline::new();
    pipeline
        .use_middleware(Box::new(TransMiddleware)) // 事务控制中间件
        .use_middleware(Box::new(ReplicaMiddleware)) // 复本同步中间件
        .use_middleware(Box::new(AofMiddleware)) // aof存储中间件
        .use_middleware(Box::new(ReplConfMasterMiddleware))
}

/* replacation */

fn handle_repl(ctx: &Arc<Context>) -> Result<()> {
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
            let mut repl_pipeline = get_repl_command_pipeline();
            conn.reset_cmd_total_bytes();
            let mut session = Session::new(conn);
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
    let conn = session.connection_mut();
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
fn get_repl_command_pipeline() -> CommandPipeline {
    let pipeline = CommandPipeline::new();
    pipeline
        .use_middleware(Box::new(TransMiddleware)) // 事务控制中间件
        .use_middleware(Box::new(ReplConfSlaveMiddleware)) // REPLCONF 命令处理中间件
        .use_middleware(Box::new(AofMiddleware)) //aof存储中间件
}
