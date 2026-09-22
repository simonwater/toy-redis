use anyhow::Result;
use bytes::{Bytes, BytesMut};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

use crate::Value;

pub struct ConnectionHandler {
    buffer: BytesMut,
    tmp_buf: [u8; 1024],
    stream: TcpStream,
}

impl ConnectionHandler {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self {
            buffer: BytesMut::with_capacity(4096),
            tmp_buf: [0u8; 1024],
            stream,
        })
    }

    pub fn send(&mut self, mut req: Value) -> Result<Value> {
        if !matches!(req, Value::Arrays(_)) {
            req = Value::Arrays(vec![req]);
        }
        self.stream.write_all(&req.to_bytes())?;
        let read_cnt = self.stream.read(&mut self.tmp_buf)?;
        self.buffer.extend_from_slice(&self.tmp_buf[..read_cnt]);
        let bytes: Bytes = self.buffer.split_to(read_cnt).freeze();
        let result = Value::from(bytes)?;
        Ok(result)
    }
}
