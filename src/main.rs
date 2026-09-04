use anyhow::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Redis is started!");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream) {
                    eprintln!("connection error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    println!("accepted new connection");
    loop {
        let mut buffer = [0; 512];
        stream.read(&mut buffer)?;
        if buffer.len() == 0 {
            break;
        }
        stream.write_all(b"+PONG\r\n").unwrap();
    }
    Ok(())
}
