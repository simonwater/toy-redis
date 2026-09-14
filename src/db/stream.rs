use anyhow::{Result, bail};
use bytes::Bytes;
use chrono::Utc;
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct EntryID {
    pub ms: i64,
    pub seq: i64,
}

impl EntryID {
    fn new(ms: i64, seq: i64) -> Self {
        Self { ms, seq }
    }

    fn next_id(&self, pat: Bytes) -> Result<Self> {
        let mut bytes = &pat[..];
        if bytes == b"0-0" {
            bail!("ERR The ID specified in XADD must be greater than 0-0")
        }
        if bytes == b"*" {
            bytes = b"*-*";
        }

        let mut ms_seq = bytes.split(|&b| b == b'-');
        if let Some(ms_bytes) = ms_seq.next() {
            let ms = if ms_bytes == b"*" {
                Utc::now().timestamp_millis().max(self.ms)
            } else {
                String::from_utf8(ms_bytes.into())?.parse::<i64>()?
            };
            if ms < self.ms {
                bail!(
                    "ERR The ID specified in XADD is equal or smaller than the target stream top item"
                );
            }

            if let Some(seq_bytes) = ms_seq.next() {
                if seq_bytes == b"*" {
                    return Ok(Self::new(ms, if ms == self.ms { self.seq + 1 } else { 0 }));
                }
                let seq = String::from_utf8(seq_bytes.into())?.parse::<i64>()?;
                if ms == self.ms && seq <= self.seq {
                    bail!(
                        "ERR The ID specified in XADD is equal or smaller than the target stream top item"
                    );
                }
                return Ok(Self::new(ms, seq));
            }
        }
        bail!("ERR XADD missing entry ID or entry ID format error");
    }

    fn to_bytes(&self) -> Bytes {
        Bytes::from(format!("{}-{}", self.ms, self.seq))
    }
}

#[derive(Debug)]
pub struct StreamEntry {
    id: EntryID,
    _value: Vec<Bytes>,
}

impl StreamEntry {
    fn new(id: EntryID, value: Vec<Bytes>) -> Self {
        Self { id, _value: value }
    }
}

#[derive(Debug)]
pub struct ReStream {
    _name: Bytes,
    stream: RwLock<Vec<StreamEntry>>,
}

impl ReStream {
    pub fn new(name: Bytes) -> Self {
        Self {
            _name: name,
            stream: RwLock::new(Vec::new()),
        }
    }

    pub fn add(&self, id: Bytes, bytes_vec: Vec<Bytes>) -> Result<Bytes> {
        let mut s = self.stream.write().unwrap();
        let last_id = s.last().map(|e| e.id).unwrap_or_else(|| EntryID::new(0, 0));
        let cur_id = last_id.next_id(id)?;
        s.push(StreamEntry::new(cur_id, bytes_vec));
        Ok(cur_id.to_bytes())
    }
}
