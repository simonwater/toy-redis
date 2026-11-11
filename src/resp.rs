use std::collections::HashSet;

use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut};

use crate::db::{Coordinates, StreamEntry};

/// 标记输入数据不完整（需凑齐一个完整的RESP帧才能解析），
/// 与畸形输入区分开，调用方可以继续读入更多数据后重试
#[derive(Debug, Clone, Copy)]
struct IncompleteInput;

impl std::fmt::Display for IncompleteInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "incomplete resp frame, need more data")
    }
}

impl std::error::Error for IncompleteInput {}

fn incomplete() -> anyhow::Error {
    anyhow::Error::new(IncompleteInput)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    SimpleStrings(String),
    SimpleErrors(String),
    Integer(i64),
    Double(f64),
    BulkStrings(Bytes),
    NullBulkStrings,
    Arrays(Vec<Value>),
    NullArrays,
    EmptyArrays,
}

impl Value {
    /// 从input中解析一个完整的RESP值。
    /// - Ok(Some((value, bytes))): 解析成功，bytes为消耗掉的原始字节
    /// - Ok(None): 数据不完整，调用方继续读入更多数据后重试
    /// - Err(e): 输入畸形
    pub fn from(input: &mut BytesMut) -> Result<Option<(Value, Bytes)>> {
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(value) => {
                let pos = parser.pos;
                let parsed_bytes = input.split_to(pos).freeze();
                Ok(Some((value, parsed_bytes)))
            }
            Err(e) => {
                if e.downcast_ref::<IncompleteInput>().is_some() {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn serilize(&self, out: &mut Vec<u8>) {
        match self {
            Value::SimpleStrings(s) => {
                out.push(b'+');
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            Value::SimpleErrors(s) => {
                out.push(b'-');
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            Value::Integer(i) => {
                out.push(b':');
                out.extend_from_slice(i.to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            Value::BulkStrings(bytes) => {
                out.push(b'$');
                out.extend_from_slice(bytes.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                out.extend_from_slice(bytes);
                out.extend_from_slice(b"\r\n");
            }
            Value::NullBulkStrings => {
                out.extend_from_slice(b"$-1\r\n");
            }
            Value::Arrays(vals) => {
                out.push(b'*');
                out.extend_from_slice(vals.len().to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
                for val in vals {
                    val.serilize(out);
                }
            }
            Value::EmptyArrays => {
                out.extend_from_slice(b"*0\r\n");
            }
            Value::NullArrays => {
                out.extend_from_slice(b"*-1\r\n");
            }
            Value::Double(num) => {
                out.push(b',');
                out.extend_from_slice(num.to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
            }
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        self.serilize(&mut bytes);
        bytes
    }

    pub fn is_string(&self) -> bool {
        matches!(
            self,
            Value::SimpleStrings(_) | Value::BulkStrings(_) | Value::NullBulkStrings
        )
    }

    pub fn to_bulk_bytes(&self) -> Result<Bytes> {
        match self {
            Value::BulkStrings(s) => Ok(s.clone()),
            _ => bail!("resp type error, expected type is bulk strings"),
        }
    }

    pub fn to_string(&self) -> Result<String> {
        match self {
            Value::SimpleStrings(s) | Value::SimpleErrors(s) => Ok(s.to_owned()),
            Value::BulkStrings(s) => Ok(String::from_utf8(s.to_vec())?),
            Value::NullBulkStrings => Ok("$-1\r\n".into()),
            Value::Integer(val) => Ok(val.to_string()),
            _ => bail!("value: {:?} can not convert to string", self),
        }
    }

    pub fn to_integer(&self) -> Result<i64> {
        match self {
            Value::Integer(val) => Ok(*val),
            Value::SimpleStrings(s) => Ok(s.parse::<i64>()?),
            Value::BulkStrings(s) => {
                let s = String::from_utf8(s.to_vec())?;
                Ok(s.parse::<i64>()?)
            }
            _ => bail!("value: {:?} can not convert to integer", self),
        }
    }

    pub fn to_double(&self) -> Result<f64> {
        match self {
            Value::Double(val) => Ok(*val),
            Value::Integer(val) => Ok(*val as f64),
            Value::SimpleStrings(s) => Ok(s.parse::<f64>()?),
            Value::BulkStrings(s) => {
                let s = String::from_utf8(s.to_vec())?;
                Ok(s.parse::<f64>()?)
            }
            _ => bail!("value: {:?} can not convert to double", self),
        }
    }

    pub fn into_array(self) -> Result<Vec<Value>> {
        match self {
            Value::Arrays(values) => Ok(values),
            _ => bail!("value: {:?} can not convert to array", self),
        }
    }
}

struct Parser<'a> {
    pos: usize,
    input: &'a mut BytesMut,
}

impl<'a> Parser<'a> {
    fn new(input: &'a mut BytesMut) -> Self {
        Self { input, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Value> {
        if self.input.is_empty() {
            return Err(incomplete());
        }
        self.pos = 0;
        self.parse_value()
    }

    fn parse_value(&mut self) -> Result<Value> {
        if self.pos >= self.input.len() {
            return Err(incomplete());
        }
        let vt = self.input[self.pos];
        match vt {
            b'+' => self.simple_string(),
            b'-' => self.simple_errors(),
            b':' => self.integer(),
            b'$' => self.bulk_string(),
            b'*' => self.arrays(),
            b',' => self.double(),
            v => bail!("unsupported type: {}", v),
        }
    }

    fn simple_string(&mut self) -> Result<Value> {
        self.consume(b'+')?;
        let s = self.parse_string()?;
        let val = Value::SimpleStrings(s);
        self.consume_terminator()?;
        Ok(val)
    }

    fn simple_errors(&mut self) -> Result<Value> {
        self.consume(b'-')?;
        let s = self.parse_string()?;
        let val = Value::SimpleErrors(s);
        self.consume_terminator()?;
        Ok(val)
    }

    fn integer(&mut self) -> Result<Value> {
        self.consume(b':')?;
        let val = self.parse_i64()?;
        self.consume_terminator()?;
        Ok(Value::Integer(val))
    }

    fn bulk_string(&mut self) -> Result<Value> {
        self.consume(b'$')?;
        let len = self.parse_i64()?;
        self.consume_terminator()?;
        if len < 0 {
            return Ok(Value::NullBulkStrings);
        }
        let len = len as usize;
        // 载荷数据不完整时等待更多数据，避免越界切片
        if len > self.input.len() - self.pos {
            return Err(incomplete());
        }
        let val = &self.input[self.pos..self.pos + len];
        let bytes = Bytes::copy_from_slice(val); // todo: 零拷贝优化·
        self.pos += len;
        if self.has_ternimator() {
            self.consume_terminator()?;
        }

        Ok(Value::BulkStrings(bytes))
    }

    fn arrays(&mut self) -> Result<Value> {
        self.consume(b'*')?;
        let len = self.parse_i64()?;
        self.consume_terminator()?;
        if len < 0 {
            return Ok(Value::NullArrays);
        }

        let len = len as usize;
        // 声明元素数超过剩余字节数时必然不完整，等待更多数据（同时避免恶意大容量分配）
        if len > self.input.len() - self.pos {
            return Err(incomplete());
        }
        let mut vals = Vec::with_capacity(len);
        for _ in 0..len {
            let val = self.parse_value()?;
            vals.push(val);
        }

        Ok(Value::Arrays(vals))
    }

    fn double(&mut self) -> Result<Value> {
        self.consume(b',')?;
        let s = self.parse_string()?;
        let num: f64 = s.parse()?;
        self.consume_terminator()?;
        Ok(Value::Double(num))
    }

    fn parse_i64(&mut self) -> Result<i64> {
        let mut flag = 1;
        let cur = self.peek();
        if cur == b'+' || cur == b'-' {
            self.pos += 1;
        }
        if cur == b'-' {
            flag = -1;
        }
        let mut val: i64 = 0;
        while !self.is_terminator() {
            if self.pos >= self.input.len() {
                return Err(incomplete());
            }
            let cur = self.advance();
            let num = (cur - b'0') as i64;
            if num < 0 || num > 9 {
                bail!("illegal integer at: {}", self.pos - 1);
            }
            if (i64::MAX - num) / 10 < val {
                bail!("integer overflow at: {}", self.pos);
            }
            val = val * 10 + num;
        }
        Ok(val * flag)
    }

    fn parse_string(&mut self) -> Result<String> {
        let start = self.pos;
        while !self.is_terminator() {
            if self.pos >= self.input.len() {
                return Err(incomplete());
            }
            self.pos += 1;
        }
        let s = String::from_utf8(self.input[start..self.pos].to_vec())?;
        Ok(s)
    }

    fn is_terminator(&self) -> bool {
        // 基于peek实现，输入末尾不越界
        self.peek_at(self.pos) == b'\r' && self.peek_at(self.pos + 1) == b'\n'
    }

    fn advance(&mut self) -> u8 {
        let cur = self.peek();
        self.pos += 1;
        cur
    }

    fn peek(&self) -> u8 {
        self.input.get(self.pos).copied().unwrap_or(b'\0')
    }

    fn peek_at(&self, pos: usize) -> u8 {
        self.input.get(pos).copied().unwrap_or(b'\0')
    }

    fn _advance_if_eq(&mut self, val: u8) -> bool {
        let cur = self.peek();
        if cur == val {
            self.pos += 1;
            return true;
        }
        return false;
    }

    // 判断当前位置是否有 \r\n 结束标志
    fn has_ternimator(&mut self) -> bool {
        self.peek_at(self.pos) == b'\r' && self.peek_at(self.pos + 1) == b'\n'
    }

    fn consume_terminator(&mut self) -> Result<()> {
        self.consume(b'\r')?;
        self.consume(b'\n')?;
        Ok(())
    }

    fn consume(&mut self, val: u8) -> Result<()> {
        if self.pos >= self.input.len() {
            return Err(incomplete());
        }
        let cur = self.peek();
        if cur != val {
            bail!(
                "expected: {:?}, but got: {:?} at position: {} for input: {:?}",
                val as char,
                cur as char,
                self.pos,
                self.input
            );
        }
        self.pos += 1;
        Ok(())
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Double(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::SimpleStrings(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::SimpleStrings(value.into())
    }
}

impl From<Bytes> for Value {
    fn from(value: Bytes) -> Self {
        Self::BulkStrings(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::Arrays(value)
    }
}

impl From<Vec<Bytes>> for Value {
    fn from(bytes_vec: Vec<Bytes>) -> Self {
        let frames: Vec<Value> = bytes_vec.into_iter().map(Value::BulkStrings).collect();
        frames.into()
    }
}

impl From<HashSet<Bytes>> for Value {
    fn from(bytes_set: HashSet<Bytes>) -> Self {
        let mut frames = Vec::with_capacity(bytes_set.len());
        for byte in bytes_set {
            frames.push(byte);
        }
        frames.into()
    }
}

impl From<Vec<StreamEntry>> for Value {
    fn from(entrys: Vec<StreamEntry>) -> Self {
        let mut frames: Vec<Value> = Vec::with_capacity(entrys.len());
        for entry in entrys {
            let id = entry.get_id();
            let values = entry.to_value();
            let mut frame: Vec<Value> = Vec::with_capacity(2);
            frame.push(Value::BulkStrings(id.to_bytes()));
            frame.push(values.into());
            frames.push(Value::Arrays(frame));
        }
        Value::Arrays(frames)
    }
}

impl From<Coordinates> for Value {
    fn from(coord: Coordinates) -> Self {
        let arr: Vec<Value> = vec![
            Bytes::from(coord.longitude.to_string()).into(),
            Bytes::from(coord.latitude.to_string()).into(),
        ];
        Value::Arrays(arr)
    }
}

impl From<Vec<Coordinates>> for Value {
    fn from(coords: Vec<Coordinates>) -> Self {
        let mut frames: Vec<Value> = Vec::with_capacity(coords.len());
        for coord in coords {
            frames.push(coord.into());
        }
        Value::Arrays(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let mut s = BytesMut::from("*1\r\n$4\r\nPING\r\n");
        let (value, _) = Value::from(&mut s).unwrap().unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn test_arr() {
        let mut s = BytesMut::from("*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Hello\r\n-World\r\n");
        let (value, _) = Value::from(&mut s).unwrap().unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn test_rdb() {
        let mut bytes_mut = BytesMut::with_capacity(1024);
        bytes_mut.extend_from_slice(b"$88\r\nREDIS0011\xfa\tredis-ver\x057.2.0\xfa\nredis-bits\xc0@\xfa\x05ctime\xc2m\x08\xbce\xfa\x08used-mem\xc2\xb0\xc4\x10\0\xfa\x08aof-base\xc0\0\xff\xf0n;\xfe\xc0\xffZ\xa2");
        let (value, _) = Value::from(&mut bytes_mut).unwrap().unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn test_incomplete() {
        // 不完整的bulk string载荷
        let mut s = BytesMut::from("$5\r\nabc");
        assert!(Value::from(&mut s).unwrap().is_none());

        // 不完整数组
        let mut s = BytesMut::from("*2\r\n$3\r\nfoo\r\n");
        assert!(Value::from(&mut s).unwrap().is_none());

        // 声明元素数超过剩余字节数
        let mut s = BytesMut::from("*10\r\n:1\r\n");
        assert!(Value::from(&mut s).unwrap().is_none());

        // 空输入
        let mut s = BytesMut::new();
        assert!(Value::from(&mut s).unwrap().is_none());
    }

    #[test]
    fn test_incremental() {
        // 分片到达: 先到一部分，补全后可正常解析
        let mut s = BytesMut::from("*2\r\n$4\r\nECH");
        assert!(Value::from(&mut s).unwrap().is_none());
        s.extend_from_slice(b"O\r\n$3\r\nfoo\r\n");
        let (value, _) = Value::from(&mut s).unwrap().unwrap();
        assert_eq!(
            value,
            Value::Arrays(vec![
                Value::BulkStrings(Bytes::from("ECHO")),
                Value::BulkStrings(Bytes::from("foo")),
            ])
        );

        // 完整帧后面跟着下一帧的开头: 只消费完整帧
        let mut s = BytesMut::from("$3\r\nfoo\r\n*2\r\n");
        let (value, consumed) = Value::from(&mut s).unwrap().unwrap();
        assert_eq!(value, Value::BulkStrings(Bytes::from("foo")));
        assert_eq!(consumed.as_ref(), b"$3\r\nfoo\r\n");
        assert_eq!(s.as_ref(), b"*2\r\n");
    }
}
