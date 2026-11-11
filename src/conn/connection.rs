use anyhow::Result;
use anyhow::bail;
use bytes::{Bytes, BytesMut};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

use crate::Command;
use crate::Value;

pub struct ConnectionHandler {
    buffer: BytesMut,
    tmp_buf: [u8; 1024],
    stream: TcpStream,
    cmd_total_bytes: usize,
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
            cmd_total_bytes: 0,
        }
    }

    // 增量读取: 当缓冲区数据不完整时继续读入，直到凑齐一个完整的RESP值
    // 目前的实现是继续读入然后重新解析，尚未实现增量解析
    pub fn receive_value(&mut self) -> Result<Option<(Value, Bytes)>> {
        loop {
            match Value::from(&mut self.buffer) {
                Ok(Some(value)) => return Ok(Some(value)),
                Ok(None) => {
                    // 缓冲区数据不完整时需要区分是否是客户端已经关闭
                    let read_cnt = self.stream.read(&mut self.tmp_buf)?;
                    if read_cnt == 0 {
                        // 对端关闭连接
                        return Ok(None);
                    }
                    self.buffer.extend_from_slice(&self.tmp_buf[..read_cnt]);
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn receive_array_values(&mut self) -> Result<Option<(Vec<Value>, Bytes)>> {
        if let Some((value, bytes)) = self.receive_value()? {
            match value {
                Value::Arrays(values) => return Ok(Some((values, bytes))),
                _ => bail!("value must be resp array."),
            }
        }
        Ok(None)
    }

    pub fn receive_command(&mut self) -> Result<Option<Command>> {
        if let Some((values, bytes)) = self.receive_array_values()? {
            let command = Command::new(values, bytes)?;
            return Ok(Some(command));
        }
        Ok(None)
    }

    pub fn receive_command_with_stat(&mut self) -> Result<Option<Command>> {
        if let Some((values, bytes)) = self.receive_array_values()? {
            self.cmd_total_bytes += bytes.len();
            let command = Command::new(values, bytes)?;
            return Ok(Some(command));
        }
        Ok(None)
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.stream.write_all(buf)?;
        Ok(())
    }

    pub fn request(&mut self, req: Value) -> Result<Option<(Value, Bytes)>> {
        self.send(req)?;
        self.receive_value()
    }

    pub fn send(&mut self, mut req: Value) -> Result<()> {
        if !matches!(req, Value::Arrays(_)) {
            req = Value::Arrays(vec![req]);
        }
        self.stream.write_all(&req.to_bytes())?;
        Ok(())
    }

    pub fn get_stream(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    pub fn reset_cmd_total_bytes(&mut self) {
        self.cmd_total_bytes = 0;
    }

    pub fn get_cmd_total_bytes(&self) -> usize {
        self.cmd_total_bytes
    }
}
