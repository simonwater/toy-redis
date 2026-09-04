use anyhow::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Redis is started!");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
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
        let read_bytes = stream.read(&mut buffer)?;
        if read_bytes == 0 {
            break;
        }
        stream.write_all(b"+PONG\r\n").unwrap();
    }
    Ok(())
}
