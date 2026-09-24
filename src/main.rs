use anyhow::{Result, bail};
use bytes::Bytes;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use toy_redis::{Command, ConnectionHandler, Context, Transaction, Value};

fn main() {
    let ctx_arc = Arc::new(Context::new());
    handle_repl(&ctx_arc).unwrap();

    let port = &ctx_arc.args_ref().port;
    println!("Server is started on port: {}!", port);
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();

    for stream in listener.incoming() {
        let ctx = ctx_arc.clone();
        println!("accepted new connection");
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    let mut connection = ConnectionHandler::from_tcp_stream(stream);
                    if let Err(e) = handle_connection(&mut connection, ctx) {
                        let res = Value::SimpleErrors(format!("{}", e));
                        println!("err: {:?}", res);
                        connection.write_all(&res.to_bytes()).unwrap();
                    }
                });
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}

fn handle_repl(ctx: &Arc<Context>) -> Result<()> {
    // 从库
    let args = ctx.args_ref();
    if let Some(addr) = args.replicaof.as_deref() {
        let mut conn = ConnectionHandler::new(addr.replace(" ", ":"))?;

        // ping
        let cmd = Value::BulkStrings("PING".into());
        let value = conn.send(cmd)?.unwrap();
        assert_eq!(value, Value::SimpleStrings("PONG".into()));

        // replconf 1
        let cmd = vec![
            Value::BulkStrings("REPLCONF".into()),
            Value::BulkStrings("listening-port".into()),
            Value::BulkStrings(Bytes::from(args.port.clone())),
        ];
        let value = conn.send(Value::Arrays(cmd))?.unwrap();
        assert_eq!(value, Value::SimpleStrings("OK".into()));

        // replconf 2
        let cmd = vec![
            Value::BulkStrings("REPLCONF".into()),
            Value::BulkStrings("capa".into()),
            Value::BulkStrings("psync2".into()),
        ];
        let value = conn.send(Value::Arrays(cmd))?.unwrap();
        assert_eq!(value, Value::SimpleStrings("OK".into()));

        // psync
        let cmd = vec![
            Value::BulkStrings("PSYNC".into()),
            Value::BulkStrings("?".into()),
            Value::BulkStrings("-1".into()),
        ];
        let _value = conn.send(Value::Arrays(cmd))?;
    }
    Ok(())
}

fn handle_connection(conn: &mut ConnectionHandler, ctx: Arc<Context>) -> Result<()> {
    let mut trans = Transaction::new();
    while let Some(input) = conn.receive_value()? {
        if let Err(e) = handle_command(input, conn, &ctx, &mut trans) {
            let res = Value::SimpleErrors(format!("{}", e));
            conn.write_all(&res.to_bytes())?;
        };
    }
    Ok(())
}

fn handle_command(
    input: Value,
    conn: &mut ConnectionHandler,
    ctx: &Arc<Context>,
    trans: &mut Transaction,
) -> Result<()> {
    let Value::Arrays(values) = input else {
        bail!("input format error!")
    };

    let cmd = Command::new(values)?;
    let output = cmd.execute(ctx, trans)?;
    output.run(conn)?;
    Ok(())
}
