use anyhow::{Context, Result, bail};
use bytes::Bytes;
use std::io::Read;

pub enum RdbItem {
    Header {
        redis_ver: u32,
    },
    Meta {
        key: String,
        value: String,
    },
    DbIndex(usize),
    HashTableSize {
        total: usize,
        expires: usize,
    },
    HashTableItem {
        key: String,
        value: String,
        expire_timestamp_ms: Option<i64>,
    },
    Checksum(u64),
}

pub struct RdbParser<R> {
    reader: R,
}

impl<R: Read> RdbParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn parse<F>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(RdbItem) -> Result<()>,
    {
        let header = self.parse_header()?;
        handler(header)?;

        loop {
            let mut marker = [0u8; 1];
            self.reader.read_exact(&mut marker)?;
            match marker[0] {
                OxFA => {
                    // 元数据
                    let meta = self.parse_meta()?;
                    handler(meta)?;
                }
                OxFE => {
                    // 数据库
                    let db_index = self.parse_length()? as u32;
                    handler(RdbItem::DbIndex(db_index as usize))?;
                }
                OxFF => {
                    // 结尾
                    let checksum = self.parse_checksum()?;
                    handler(RdbItem::Checksum(checksum))?;
                    break;
                }
                other => {
                    bail!("invalid rdb marker: 0x{:02X}", other)
                }
            }
        }

        Ok(())
    }

    fn parse_header(&mut self) -> Result<RdbItem> {
        let mut magic_buf = [0u8; 5];
        self.reader.read_exact(&mut magic_buf)?;
        if &magic_buf != b"REDIS" {
            bail!("invalid magic!")
        }

        let mut ver_buf = [0u8; 4];
        self.reader.read_exact(&mut ver_buf)?;
        let ver_str = std::str::from_utf8(&ver_buf).context("版本号非 UTF-8")?;
        let redis_ver = ver_str.parse().unwrap_or(11);
        Ok(RdbItem::Header { redis_ver })
    }

    fn parse_meta(&mut self) -> Result<RdbItem> {
        let key = self.parse_string()?;
        let value = self.parse_string()?;
        Ok(RdbItem::Meta { key, value })
    }

    fn parse_db(&mut self) {}

    fn parse_length(&mut self) -> Result<u64> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        let first = buf[0];
        let two_bits = first >> 6;
        match two_bits {
            0 => Ok((first & 0x3F) as u64), // 6bits
            1 => {
                // 14bits
                let mut buf = [0u8; 1];
                self.reader.read_exact(&mut buf)?;
                let ans = ((first & 0x3f) as u16) << 8 | (buf[0] as u16);
                Ok(ans as u64)
            }
            2 => {
                // 4bytes
                let mut buf = [0u8; 4];
                self.reader.read_exact(&mut buf)?;
                let ans = u32::from_be_bytes(buf);
                Ok(ans as u64)
            }
            v => bail!("unsupported length flag: {}!", v),
        }
    }

    fn parse_bytes(&mut self) -> Result<Bytes> {
        let len = self.parse_length()?;
        let mut buf = vec![0u8; len as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(Bytes::from(buf))
    }

    fn parse_string(&mut self) -> Result<String> {
        let bytes = self.parse_bytes()?;
        let s = String::from_utf8(bytes.to_vec())?;
        Ok(s)
    }

    fn parse_checksum(&mut self) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.reader.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }
}
