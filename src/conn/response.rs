use anyhow::Result;
use bytes::Bytes;
use std::io::Read;
use std::sync::Arc;

use crate::{Context, Session, Value};

pub enum ConnectionState {
    KeepAlive(Session),
    Close,
}

/// 将命令的执行和返回结果的逻辑解藕，命令执行过程只关心业务逻辑
pub enum CommandResponse {
    Standard(Value), // 标准的保持连接并等待一下个命令
    TakeOver(Box<dyn FnOnce(&Arc<Context>, Session) -> Result<ConnectionState>>), // 由自定义的逻辑接管连接
    Close(Value),                                                                 // 结束连接
    ReplConf(Value), // 副本响应主服务replconf命令时返回的结果，区别于Standard，副本同步返回Standard时不会给主服务响应
    ReplSync {
        init_value: Value,
        stream: Box<dyn Read>,
        len: usize,
    },
}

impl CommandResponse {
    pub fn run(self, ctx: &Arc<Context>, mut session: Session) -> Result<ConnectionState> {
        let conn = session.connection_mut()?;
        match self {
            Self::Standard(value) | Self::ReplConf(value) => {
                conn.write_all(&value.to_bytes())?;
                Ok(ConnectionState::KeepAlive(session))
            }
            Self::TakeOver(handler) => handler(ctx, session),
            Self::Close(value) => {
                conn.write_all(&value.to_bytes())?;
                Ok(ConnectionState::Close)
            }
            Self::ReplSync {
                init_value,
                mut stream,
                len,
            } => {
                conn.write_all(&init_value.to_bytes())?;
                let head = format!("${}\r\n", len);
                conn.write_all(head.as_bytes())?;
                std::io::copy(&mut stream, conn.get_stream())?;
                ctx.repl_hub().register_repla(session);
                Ok(ConnectionState::Close)
            }
        }
    }
}

impl From<Value> for CommandResponse {
    fn from(value: Value) -> Self {
        Self::Standard(value)
    }
}

impl From<i64> for CommandResponse {
    fn from(value: i64) -> Self {
        Self::Standard(value.into())
    }
}

impl From<f64> for CommandResponse {
    fn from(value: f64) -> Self {
        Self::Standard(value.into())
    }
}

impl From<String> for CommandResponse {
    fn from(value: String) -> Self {
        Self::Standard(value.into())
    }
}

impl From<&str> for CommandResponse {
    fn from(value: &str) -> Self {
        Self::Standard(value.into())
    }
}

impl From<Bytes> for CommandResponse {
    fn from(value: Bytes) -> Self {
        Self::Standard(value.into())
    }
}

impl From<Vec<Value>> for CommandResponse {
    fn from(value: Vec<Value>) -> Self {
        Self::Standard(value.into())
    }
}

impl From<Vec<Bytes>> for CommandResponse {
    fn from(bytes_vec: Vec<Bytes>) -> Self {
        let frames: Vec<Value> = bytes_vec.into_iter().map(Value::BulkStrings).collect();
        frames.into()
    }
}
