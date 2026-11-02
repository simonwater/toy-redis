use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use toy_redis::{ConnectionHandler, Context, main_starter, repl_starter};

fn main() {
    let ctx_arc = Arc::new(Context::init().expect("redis context init error"));
    let port = &ctx_arc.args_ref().port;
    println!("Server is started on port: {}!", port);

    // 主从同步副本启动逻辑
    repl_starter::start(&ctx_arc).unwrap();
    // aof重放
    let aof_handler = ctx_arc.aof_handler();
    aof_handler.replay(&ctx_arc).expect("aof replay error");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    for stream in listener.incoming() {
        let ctx = ctx_arc.clone();
        println!("accepted new connection");
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    let conn = ConnectionHandler::from_tcp_stream(stream);
                    if let Err(e) = main_starter::start(ctx, conn) {
                        eprintln!("err: {:?}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("err: {:?}", e);
            }
        }
    }
}
