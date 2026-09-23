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
        Ok(Self::from_tcp_stream(stream))
    }

    pub fn from_tcp_stream(stream: TcpStream) -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
            tmp_buf: [0u8; 1024],
            stream,
        }
    }

    pub fn receive_value(&mut self) -> Result<Option<Value>> {
        let read_cnt = self.stream.read(&mut self.tmp_buf)?;
        if read_cnt == 0 {
            return Ok(None);
        }
        self.buffer.extend_from_slice(&self.tmp_buf[..read_cnt]);
        let bytes: Bytes = self.buffer.split_to(read_cnt).freeze();
        let input = Value::from(bytes)?;
        Ok(Some(input))
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.stream.write_all(buf)?;
        Ok(())
    }

    pub fn send(&mut self, mut req: Value) -> Result<Option<Value>> {
        if !matches!(req, Value::Arrays(_)) {
            req = Value::Arrays(vec![req]);
        }
        self.stream.write_all(&req.to_bytes())?;
        self.receive_value()
    }
}
