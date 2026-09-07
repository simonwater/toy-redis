use anyhow::{Result, bail};
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

fn handle_connection(stream: &mut TcpStream, db: Arc<RwLock<MemoryDB>>) -> Result<()> {
    let mut buffer = [0; 1024];
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
        bail!("input format error!")
    };
    let cmd = Command::new(values)?;
    cmd.execute(db)
}
