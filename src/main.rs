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
                        eprintln!("connection error: {}", e);
                        let res = Value::SimpleErrors(format!("{}", e));
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
        let input = Value::from(bytes)?;
        let output = execute(input, &db)?;

        let mut out = Vec::with_capacity(128);
        output.serilize(&mut out);
        stream.write_all(&out).unwrap();
    }
}

fn execute(input: Value, db: &Arc<MemoryDB>) -> Result<Value> {
    let Value::Arrays(values) = input else {
        bail!("input format error!")
    };
    let cmd = Command::new(values)?;
    cmd.execute(db)
}
