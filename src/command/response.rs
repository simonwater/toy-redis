use anyhow::Result;
use bytes::Bytes;
use std::io::Read;
use std::sync::Arc;

use crate::{ConnectionHandler, Context, Value};

pub enum ConnectionState {
    KeepAlive(ConnectionHandler),
    TakenOver,
}

pub enum CommandResponse {
    RespValue(Value),
    Stream {
        init_value: Value,
        stream: Box<dyn Read>,
        len: usize,
    },
    Replication {
        init_value: Value,
        stream: Box<dyn Read>,
        len: usize,
    },
}

impl CommandResponse {
    pub fn run(self, mut conn: ConnectionHandler, ctx: &Arc<Context>) -> Result<ConnectionState> {
        match self {
            Self::RespValue(value) => {
                conn.write_all(&value.to_bytes())?;
                Ok(ConnectionState::KeepAlive(conn))
            }
            Self::Stream {
                init_value,
                mut stream,
                len,
            } => {
                conn.write_all(&init_value.to_bytes())?;
                let head = format!("${}\r\n", len);
                conn.write_all(head.as_bytes())?;
                std::io::copy(&mut stream, conn.get_stream())?;
                Ok(ConnectionState::TakenOver)
            }
            Self::Replication {
                init_value,
                mut stream,
                len,
            } => {
                conn.write_all(&init_value.to_bytes())?;
                let head = format!("${}\r\n", len);
                conn.write_all(head.as_bytes())?;
                std::io::copy(&mut stream, conn.get_stream())?;
                ctx.repl_hub().register_repla(conn);
                Ok(ConnectionState::TakenOver)
            }
        }
    }
}

impl From<Value> for CommandResponse {
    fn from(value: Value) -> Self {
        Self::RespValue(value)
    }
}

impl From<i64> for CommandResponse {
    fn from(value: i64) -> Self {
        Self::RespValue(value.into())
    }
}

impl From<f64> for CommandResponse {
    fn from(value: f64) -> Self {
        Self::RespValue(value.into())
    }
}

impl From<String> for CommandResponse {
    fn from(value: String) -> Self {
        Self::RespValue(value.into())
    }
}

impl From<&str> for CommandResponse {
    fn from(value: &str) -> Self {
        Self::RespValue(value.into())
    }
}

impl From<Bytes> for CommandResponse {
    fn from(value: Bytes) -> Self {
        Self::RespValue(value.into())
    }
}

impl From<Vec<Value>> for CommandResponse {
    fn from(value: Vec<Value>) -> Self {
        Self::RespValue(value.into())
    }
}

impl From<Vec<Bytes>> for CommandResponse {
    fn from(bytes_vec: Vec<Bytes>) -> Self {
        let frames: Vec<Value> = bytes_vec.into_iter().map(Value::BulkStrings).collect();
        frames.into()
    }
}
