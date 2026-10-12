use anyhow::{Result, bail};
use bytes::Bytes;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use toy_redis::{
    CommandPipeline, ConnectionHandler, ConnectionState, Context, ReplicaMiddleware,
    TransMiddleware, Transaction, Value,
};

fn main() {
    let ctx_arc = Arc::new(Context::new());
    let port = &ctx_arc.args_ref().port;
    println!("Server is started on port: {}!", port);

    handle_repl(&ctx_arc).unwrap();
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();

    for stream in listener.incoming() {
        let ctx = ctx_arc.clone();
        println!("accepted new connection");
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    let connection = ConnectionHandler::from_tcp_stream(stream);
                    if let Err(e) = handle_connection(connection, ctx) {
                        eprintln!("err: {:?}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("err: {}", e);
            }
        }
    }
}

fn handle_connection(mut conn: ConnectionHandler, ctx: Arc<Context>) -> Result<()> {
    let mut trans = Transaction::new();
    let cmd_pipeline = get_command_pipeline();
    // 读取命令
    while let Some(command) = conn.receive_command()? {
        // 执行命令
        let response = match cmd_pipeline.execute(&command, &ctx, &mut trans) {
            Ok(res) => res,
            Err(e) => Value::SimpleErrors(format!("{}", e)).into(),
        };

        // 回写响应
        match response.run(conn, &ctx)? {
            ConnectionState::KeepAlive(keep_conn) => conn = keep_conn,
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
        let _init_value = conn.send(Value::Arrays(cmd))?;
        let _rdb_stream = conn.receive_value()?;
        let ctx = ctx.clone();
        thread::spawn(move || {
            let mut trans = Transaction::new();
            let mut repl_pipeline = get_repl_command_pipeline();
            // 增量命令
            loop {
                if let Err(err) =
                    handle_repl_command(&mut repl_pipeline, &mut conn, &ctx, &mut trans)
                {
                    eprint!("Replica synchronization: {}", err);
                    return;
                }
            }
        });
    }
    Ok(())
}

fn handle_repl_command(
    pipeline: &mut CommandPipeline,
    conn: &mut ConnectionHandler,
    ctx: &Arc<Context>,
    trans: &mut Transaction,
) -> Result<()> {
    if let Some(command) = conn.receive_command()? {
        let _ = pipeline.execute(&command, &ctx, trans)?;
        Ok(())
    } else {
        bail!("connection is lost.")
    }
}

fn get_repl_command_pipeline() -> CommandPipeline {
    let pipeline = CommandPipeline::new();
    pipeline.use_middleware(Box::new(TransMiddleware)) // 事务控制中间件
}
