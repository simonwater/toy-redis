use anyhow::Result;
use bytes::Bytes;
use std::io::Read;

use crate::{ConnectionHandler, Value};

pub enum CommandResponse {
    RespValue(Value),
    Stream {
        init_value: Value,
        stream: Box<dyn Read>,
        len: usize,
    },
}

impl CommandResponse {
    pub fn run(self, conn: &mut ConnectionHandler) -> Result<()> {
        match self {
            Self::RespValue(value) => {
                conn.write_all(&value.to_bytes())?;
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
            }
        };
        Ok(())
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
