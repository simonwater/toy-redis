use anyhow::{Context, Result, bail};
use bytes::Bytes;
use std::io::{Read, Seek, SeekFrom};

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
        hash_size: usize,
        expire_size: usize,
    },
    HashTableItem {
        key: String,
        value: RdbValue,
        expire_at_ms: Option<u64>,
    },
    Checksum(u64),
}

pub enum RdbValue {
    String(Bytes),
}

pub struct RdbParser<R> {
    reader: R,
}

impl<R: Read + Seek> RdbParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn parse<F>(&mut self, mut evt_handler: F) -> Result<()>
    where
        F: Fn(RdbItem) -> Result<()>,
    {
        let redis_ver = self.parse_header()?;
        evt_handler(RdbItem::Header { redis_ver })?;

        loop {
            let mut marker = [0u8; 1];
            self.reader.read_exact(&mut marker)?;
            match marker[0] {
                OxFA => {
                    // 元数据
                    let (key, value) = self.parse_meta()?;
                    evt_handler(RdbItem::Meta { key, value })?;
                }
                OxFE => {
                    // 数据库
                    let db_index = self.parse_length()? as u32;
                    evt_handler(RdbItem::DbIndex(db_index as usize))?;

                    self.parse_db(&mut evt_handler)?;
                }
                OxFF => {
                    // 结尾
                    let checksum = self.parse_checksum()?;
                    evt_handler(RdbItem::Checksum(checksum))?;
                    break;
                }
                other => {
                    bail!("invalid rdb marker: 0x{:02X}", other)
                }
            }
        }

        Ok(())
    }

    fn parse_header(&mut self) -> Result<u32> {
        let mut magic_buf = [0u8; 5];
        self.reader.read_exact(&mut magic_buf)?;
        if &magic_buf != b"REDIS" {
            bail!("invalid magic!")
        }

        let mut ver_buf = [0u8; 4];
        self.reader.read_exact(&mut ver_buf)?;
        let ver_str = std::str::from_utf8(&ver_buf).context("版本号非 UTF-8")?;
        let redis_ver = ver_str.parse().unwrap_or(11);
        Ok(redis_ver)
    }

    fn parse_meta(&mut self) -> Result<(String, String)> {
        let key = self.parse_string()?;
        let value = self.parse_string()?;
        Ok((key, value))
    }

    fn parse_db<F>(&mut self, evt_handler: &mut F) -> Result<()>
    where
        F: Fn(RdbItem) -> Result<()>,
    {
        // 哈希表大小
        let mut flag = [0u8; 1];
        self.reader.read_exact(&mut flag)?;
        if flag[0] == 0xFB {
            let hash_size = self.parse_length()? as usize;
            let expire_size = self.parse_length()? as usize;
            evt_handler(RdbItem::HashTableSize {
                hash_size,
                expire_size,
            })?;
        } else {
            // 回退一个字节
            self.reader.seek(SeekFrom::Current(-1))?;
        }

        loop {
            let mut marker = [0u8; 1];
            self.reader.read_exact(&mut marker)?;

            let mut expire_at_ms: Option<u64> = None;
            // 执行完 match 后marker中放着value type
            match marker[0] {
                0xFC => {
                    // 毫秒过期时间
                    let mut buf = [0u8; 8];
                    self.reader.read_exact(&mut buf)?;
                    expire_at_ms = Some(u64::from_le_bytes(buf));

                    self.reader.read_exact(&mut marker)?;
                }
                0xFD => {
                    // 秒过期时间
                    let mut buf = [0u8; 4];
                    self.reader.read_exact(&mut buf)?;
                    let secs = u32::from_le_bytes(buf) as u64;
                    expire_at_ms = Some(secs * 1000);

                    self.reader.read_exact(&mut marker)?;
                }
                0xFE | 0xFF => {
                    self.reader.seek(SeekFrom::Current(-1))?;
                    return Ok(());
                }
                _ => {}
            };

            let type_byte = marker[0];
            let key = self.parse_string()?;
            let value = self.parse_value(type_byte)?;

            evt_handler(RdbItem::HashTableItem {
                key,
                value,
                expire_at_ms,
            })?;
        }
    }

    fn parse_value(&mut self, type_byte: u8) -> Result<RdbValue> {
        match type_byte {
            0 => {
                // 0x00 代表 String
                let s = self.parse_bytes()?;
                Ok(RdbValue::String(s))
            }
            other => bail!("unsupported RDB Value type: 0x{:02X}", other),
        }
    }

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
