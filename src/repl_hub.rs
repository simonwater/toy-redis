use crate::{ConnectionHandler, Session};
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum ReplMsg {
    Propagation(Bytes),
    GetAck {
        respond: Arc<Sender<i64>>,
        timeout: Duration,
    },
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
            let conn = session.connection_mut();
            loop {
                match receiver.recv() {
                    Ok(cmd) => match cmd {
                        ReplMsg::Propagation(bytes) => Self::handle_prop_command(bytes, conn),
                        ReplMsg::GetAck { respond, timeout } => {
                            Self::handle_ack_msg(respond, timeout, conn)
                        }
                    },
                    Err(e) => {
                        eprintln!("propagation channel error: {}", e);
                        break;
                    }
                };
            }
        });
    }

    fn handle_prop_command(bytes: Bytes, conn: &mut ConnectionHandler) {
        if let Err(e) = conn.write_all(&bytes.slice(..)) {
            eprintln!(
                "error when send propagation command to the replication: {}",
                e
            );
        }
    }

    fn handle_ack_msg(respond: Arc<Sender<i64>>, timeout: Duration, conn: &mut ConnectionHandler) {
        conn.get_stream().set_read_timeout(Some(timeout)).unwrap();
        match Self::get_ack(conn) {
            Ok(offset) => {
                if let Err(e) = respond.send(offset) {
                    eprintln!("error when respond ack to main thread: {}", e);
                }
            }
            Err(e) => {
                eprintln!("error when send `GETACK` to the replication: {}", e)
            }
        }
        conn.get_stream().set_read_timeout(None).unwrap();
    }

    fn get_ack(conn: &mut ConnectionHandler) -> Result<i64> {
        let bytes_vec = vec![
            Bytes::from_static(b"REPLCONF"),
            Bytes::from_static(b"GETACK"),
            Bytes::from_static(b"*"),
        ];
        if let Some((value, _)) = conn.request(bytes_vec.into())? {
            let values = value.into_array()?;
            return values.last().unwrap().to_integer();
        }
        // 返回None时对端关闭了连接
        bail!("replication server is unresponsive and has shut down!")
    }

    pub fn prop_replmsg(&self, cmd: ReplMsg) {
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
}
