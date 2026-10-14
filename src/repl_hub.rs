use crate::{ConnectionHandler, Session, Value};
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::mpsc::{self, Sender};
use std::thread;

#[derive(Clone)]
pub enum ReplMsg {
    Progation(Bytes),
    GetAck(Arc<Sender<Result<i64>>>),
}

pub struct ReplHub {
    repl_senders: RwLock<Vec<Sender<ReplMsg>>>,
}

impl ReplHub {
    pub fn new() -> Self {
        Self {
            repl_senders: RwLock::new(Vec::with_capacity(8)),
        }
    }

    /// 注册新副本，增加管理副本连接的工作线程
    pub fn register_repla(&self, mut session: Session) {
        let (sender, receiver) = mpsc::channel();
        self.repl_senders.write().unwrap().push(sender);
        thread::spawn(move || {
            let mut conn = session.connection_mut();
            loop {
                match receiver.recv() {
                    Ok(cmd) => match cmd {
                        ReplMsg::Progation(bytes) => {
                            if let Err(e) = conn.write_all(&bytes.slice(..)) {
                                eprintln!("error when send progation command to the repla: {}", e);
                                break;
                            }
                        }
                        ReplMsg::GetAck(sender) => {
                            let ack = Self::handle_ack(&mut conn);
                            if let Err(e) = sender.send(ack) {
                                eprintln!("error when send `GETACK` command to the repla: {}", e);
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("repla channel error: {}", e);
                        break;
                    }
                };
            }
        });
    }

    pub fn prop_command(&self, cmd: ReplMsg) {
        let senders = self.repl_senders.read().unwrap();
        for sender in senders.iter() {
            sender.send(cmd.clone()).unwrap();
        }
    }

    pub fn len(&self) -> usize {
        self.repl_senders.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.repl_senders.read().unwrap().is_empty()
    }

    fn handle_ack(conn: &mut ConnectionHandler) -> Result<i64> {
        let values: Vec<Value> = vec!["REPLCONF".into(), "GETACK".into(), "*".into()];
        if let Some((value, _)) = conn.request(values.into())? {
            return value.to_integer();
        }
        bail!("replication server connection is lost!")
    }
}
