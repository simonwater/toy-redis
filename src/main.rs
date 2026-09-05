use anyhow::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use toy_redis::Value;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Redis is started!");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    // todo thread pool
                    if let Err(e) = handle_connection(stream) {
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

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    println!("accepted new connection");
    let mut buffer = [0; 512];
    loop {
        let read_cnt = stream.read(&mut buffer)?;
        if read_cnt == 0 {
            break;
        }
        let input = Value::from(&buffer[..read_cnt])?;
        println!("input is: {:?}", input);
        let output = execute(input)?;

        let mut out = Vec::with_capacity(128);
        output.serilize(&mut out);
        stream.write_all(&out).unwrap();
    }
    Ok(())
}

fn execute(input: Value) -> Result<Value> {
    match input {
        Value::Arrays(values) => {
            let mut iter = values.iter();
            let Some(cmd_val) = iter.next() else {
                return Ok(Value::SimpleErrors("missing command!".into()));
            };
            match cmd_val {
                Value::BulkStrings(s) | Value::SimpleStrings(s) => {
                    let cmd = s.to_uppercase();
                    match cmd.as_str() {
                        "PING" => return Ok(Value::SimpleStrings("PONG".into())),
                        "ECHO" => {
                            let arg = iter.next().unwrap();
                            return Ok(arg.clone());
                        }
                        _ => return Ok(Value::SimpleErrors("unsported command!".into())),
                    }
                }
                _ => {
                    return Ok(Value::SimpleErrors("command format error!".into()));
                }
            }
        }
        _ => {
            return Ok(Value::SimpleErrors("input format error!".into()));
        }
    }
}
