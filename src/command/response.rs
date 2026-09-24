use anyhow::Result;
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
            Self::Stream { init_value, .. } => {
                conn.write_all(&init_value.to_bytes())?;
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
