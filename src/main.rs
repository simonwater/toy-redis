use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use toy_redis::{Command, Context, Transaction, Value};

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
            Ok(mut stream) => {
                thread::spawn(move || {
                    if let Err(e) = handle_connection(&mut stream, ctx) {
                        let res = Value::SimpleErrors(format!("{}", e));
                        println!("err: {:?}", res);
                        stream.write_all(&res.to_bytes()).unwrap();
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
    if let Some(addr) = ctx.args_ref().replicaof.as_deref() {
        let mut stream = TcpStream::connect(addr.replace(" ", ":"))?;
        let mut buffer = BytesMut::with_capacity(4096);
        let mut tmp_buf = [0u8; 1024];

        // ping
        let cmd_name = Value::BulkStrings("PING".into());
        let bytes = Value::Arrays(vec![cmd_name]);
        stream.write_all(&bytes.to_bytes())?;
        let read_cnt = stream.read(&mut tmp_buf)?;
        buffer.extend_from_slice(&tmp_buf[..read_cnt]);
        let bytes: Bytes = buffer.split_to(read_cnt).freeze();
        let value = Value::from(bytes)?;
        assert_eq!(value, Value::SimpleStrings("PONG".into()));
    }
    Ok(())
}

fn handle_connection(stream: &mut TcpStream, db: Arc<Context>) -> Result<()> {
    let mut buffer = BytesMut::with_capacity(4096);
    let mut tmp_buf = [0u8; 1024];
    let mut trans = Transaction::new();

    loop {
        let read_cnt = stream.read(&mut tmp_buf)?;
        if read_cnt == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&tmp_buf[..read_cnt]);
        let bytes: Bytes = buffer.split_to(read_cnt).freeze();

        let output = match handle_command(bytes, &db, &mut trans) {
            Ok(output) => output,
            Err(e) => Value::SimpleErrors(format!("{}", e)),
        };

        stream.write_all(&output.to_bytes())?;
    }
}

fn handle_command(bytes: Bytes, ctx: &Arc<Context>, trans: &mut Transaction) -> Result<Value> {
    let input = Value::from(bytes)?;
    let Value::Arrays(values) = input else {
        bail!("input format error!")
    };

    let cmd = Command::new(values)?;
    let res = cmd.execute(ctx, trans)?;
    Ok(res)
}
