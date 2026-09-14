use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use toy_redis::{Command, MemoryDB, Value};

fn main() {
    let db_rc: Arc<MemoryDB> = Arc::new(MemoryDB::new());
    println!("Redis is started!");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        let db = db_rc.clone();
        println!("accepted new connection");
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || {
                    // todo thread pool
                    if let Err(e) = handle_connection(&mut stream, db) {
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

fn handle_connection(stream: &mut TcpStream, db: Arc<MemoryDB>) -> Result<()> {
    let mut buffer = BytesMut::with_capacity(4096);
    let mut tmp_buf = [0u8; 1024];
    loop {
        let read_cnt = stream.read(&mut tmp_buf)?;
        if read_cnt == 0 {
            bail!("input is empty!")
        }
        buffer.extend_from_slice(&tmp_buf[..read_cnt]);
        let bytes: Bytes = buffer.split_to(read_cnt).freeze();

        let output = match handle_request(bytes, &db) {
            Ok(output) => output,
            Err(e) => Value::SimpleErrors(format!("{}", e)),
        };

        stream.write_all(&output.to_bytes())?;
    }
}

fn handle_request(bytes: Bytes, db: &Arc<MemoryDB>) -> Result<Value> {
    let input = Value::from(bytes)?;
    let Value::Arrays(values) = input else {
        bail!("input format error!")
    };
    let cmd = Command::new(values)?;
    let res = cmd.execute(db)?;
    Ok(res)
}
