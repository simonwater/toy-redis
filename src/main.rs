use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use toy_redis::{Command, Context, Transaction, Value};

fn main() {
    let ctx_arc = Arc::new(Context::new());
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
