use crate::Value;
use std::sync::RwLock;
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::ConnectionHandler;

pub struct ReplHub {
    repl_senders: RwLock<Vec<Sender<Value>>>,
}

impl ReplHub {
    pub fn new() -> Self {
        Self {
            repl_senders: RwLock::new(Vec::with_capacity(8)),
        }
    }

    /// 注册新副本，增加管理副本连接的工作线程
    pub fn register_repla(&self, mut conn: ConnectionHandler) {
        let (sender, receiver) = mpsc::channel();
        self.repl_senders.write().unwrap().push(sender);
        thread::spawn(move || {
            loop {
                match receiver.recv() {
                    Ok(msg) => {
                        if let Err(e) = conn.send(msg) {
                            eprintln!("repla connection error when sync command: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("repla channel error: {}", e);
                        break;
                    }
                };
            }
        });
    }

    pub fn prop_command(&self, cmd: Value) {
        let senders = self.repl_senders.read().unwrap();
        for sender in senders.iter() {
            sender.send(cmd.clone()).unwrap();
        }
    }
}
