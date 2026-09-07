use anyhow::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, RwLock};
use std::thread;
use toy_redis::{Command, MemoryDB, Value};

fn main() {
    let db_rc: Arc<RwLock<MemoryDB>> = Arc::new(RwLock::new(MemoryDB::new()));
    println!("Redis is started!");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        let db = db_rc.clone();
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    // todo thread pool
                    if let Err(e) = handle_connection(stream, db) {
                        eprintln!("connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, db: Arc<RwLock<MemoryDB>>) -> Result<()> {
    println!("accepted new connection");
    let mut buffer = [0; 512];
    loop {
        let read_cnt = stream.read(&mut buffer)?;
        if read_cnt == 0 {
            break;
        }
        let input = Value::from(&buffer[..read_cnt])?;
        //println!("input is: {:?}", input);
        let output = execute(input, &db)?;
        //println!("output is: {:?}", output);

        let mut out = Vec::with_capacity(128);
        output.serilize(&mut out);
        stream.write_all(&out).unwrap();
    }
    Ok(())
}

fn execute(input: Value, db: &Arc<RwLock<MemoryDB>>) -> Result<Value> {
    let Value::Arrays(values) = input else {
        return Ok(Value::SimpleErrors("input format error!".into()));
    };
    let cmd = match Command::new(values) {
        Ok(cmd) => cmd,
        Err(e) => return Ok(Value::SimpleErrors(format!("{}", e))),
    };
    cmd.execute(db)
}
