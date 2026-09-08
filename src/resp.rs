use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub enum Value {
    SimpleStrings(String),
    SimpleErrors(String),
    Integer(i64),
    BulkStrings(String),
    NullBulkStrings,
    Arrays(Vec<Value>),
    NullArrays,
}

impl Value {
    pub fn from(input: &[u8]) -> Result<Value> {
        let mut parser = Parser::new(input);
        parser.parse()
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
            Value::BulkStrings(s) => {
                let bytes = s.as_bytes();
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
            Value::NullArrays => {
                out.extend_from_slice(b"*-1\r\n");
            }
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(512);
        self.serilize(&mut bytes);
        bytes
    }

    pub fn is_string(&self) -> bool {
        matches!(
            self,
            Value::SimpleStrings(_) | Value::BulkStrings(_) | Value::NullBulkStrings
        )
    }

    pub fn into_string(self) -> Result<String> {
        match self {
            Value::SimpleStrings(s) | Value::BulkStrings(s) | Value::SimpleErrors(s) => Ok(s),
            Value::NullBulkStrings => Ok("$-1\r\n".into()),
            Value::Integer(val) => Ok(val.to_string()),
            _ => bail!("can not convert to string"),
        }
    }

    pub fn into_integer(self) -> Result<i64> {
        match self {
            Value::Integer(val) => Ok(val),
            Value::SimpleStrings(s) | Value::BulkStrings(s) => Ok(s.parse::<i64>()?),
            _ => bail!("can not convert to integer"),
        }
    }
}

struct Parser<'a> {
    pos: usize,
    input: &'a [u8],
}

impl<'a> Parser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Value> {
        if self.input.is_empty() {
            bail!("can not parse empty input")
        }
        self.pos = 0;
        self.parse_value()
    }

    fn parse_value(&mut self) -> Result<Value> {
        let vt = self.input[self.pos];
        match vt {
            b'+' => self.simple_string(),
            b'-' => self.simple_errors(),
            b':' => self.integer(),
            b'$' => self.bulk_string(),
            b'*' => self.arrays(),
            v => bail!("unsported type: {}", v),
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
        if len < 0 {
            return Ok(Value::NullBulkStrings);
        }
        let len = len as usize;
        self.consume_terminator()?;
        let val = String::from_utf8(self.input[self.pos..self.pos + len].to_vec())?;
        self.pos += len;
        self.consume_terminator()?;

        Ok(Value::BulkStrings(val))
    }

    fn arrays(&mut self) -> Result<Value> {
        self.consume(b'*')?;
        let len = self.parse_i64()?;
        self.consume_terminator()?;
        if len < 0 {
            return Ok(Value::NullArrays);
        }

        let len = len as usize;
        let mut vals = Vec::with_capacity(len);
        for _ in 0..len {
            let val = self.parse_value()?;
            vals.push(val);
        }

        Ok(Value::Arrays(vals))
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
            self.pos += 1;
        }
        let s = String::from_utf8(self.input[start..self.pos].to_vec())?;
        Ok(s)
    }

    fn is_terminator(&self) -> bool {
        &self.input[self.pos..self.pos + 2] == b"\r\n"
    }

    fn advance(&mut self) -> u8 {
        let cur = self.peek();
        self.pos += 1;
        cur
    }

    fn peek(&self) -> u8 {
        self.input.get(self.pos).copied().unwrap_or(b'\0')
    }

    fn _advance_if_eq(&mut self, val: u8) -> bool {
        let cur = self.peek();
        if cur == val {
            self.pos += 1;
            return true;
        }
        return false;
    }

    fn consume_terminator(&mut self) -> Result<()> {
        self.consume(b'\r')?;
        self.consume(b'\n')?;
        Ok(())
    }

    fn consume(&mut self, val: u8) -> Result<()> {
        let cur = self.peek();
        if cur != val {
            bail!(
                "expected: {}, but got: {} at position: {}",
                val as char,
                cur as char,
                self.pos
            );
        }
        self.pos += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let s = b"*1\r\n$4\r\nPING\r\n";
        let value = Value::from(s).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn test_arr() {
        let s = b"*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Hello\r\n-World\r\n";
        let value = Value::from(s).unwrap();
        println!("{:?}", value);
    }
}
