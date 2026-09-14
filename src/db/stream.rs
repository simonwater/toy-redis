use anyhow::{Ok, Result, bail};
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

    fn from(bytes: Bytes, default_seq: i64) -> Result<Self> {
        let mut ms_seq = bytes.split(|&b| b == b'-');
        let ms_bytes = ms_seq.next().unwrap();
        let ms: i64 = String::from_utf8(ms_bytes.into())?.parse()?;
        let seq = if let Some(seq_bytes) = ms_seq.next() {
            String::from_utf8(seq_bytes.into())?.parse()?
        } else {
            default_seq
        };
        Ok(Self { ms, seq })
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

    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(format!("{}-{}", self.ms, self.seq))
    }
}

#[derive(Debug, Clone)]
pub struct StreamEntry {
    id: EntryID,
    value: Vec<Bytes>,
}

impl StreamEntry {
    fn new(id: EntryID, value: Vec<Bytes>) -> Self {
        Self { id, value }
    }

    pub fn get_id(&self) -> EntryID {
        self.id
    }

    pub fn to_value(self) -> Vec<Bytes> {
        self.value
    }
}

#[derive(Debug)]
pub struct ReStream {
    _name: Bytes,
    stream: RwLock<Vec<StreamEntry>>,
}

impl ReStream {
    fn lower_bound(&self, target: EntryID) -> usize {
        let datas = self.stream.read().unwrap();
        let mut ans = datas.len();
        if datas.is_empty() {
            return ans;
        }
        let mut lo = 0;
        let mut hi = datas.len() - 1;
        while lo <= hi {
            let mid = lo + ((hi - lo) >> 1);
            let mid_val = &datas[mid];
            if mid_val.id >= target {
                ans = mid;
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            } else {
                lo = mid + 1;
            }
        }
        ans
    }

    fn high_bound(&self, target: EntryID) -> usize {
        let datas = self.stream.read().unwrap();
        let mut ans = datas.len();
        if datas.is_empty() {
            return ans;
        }
        let mut lo = 0;
        let mut hi = datas.len() - 1;
        while lo <= hi {
            let mid = lo + ((hi - lo) >> 1);
            let mid_val = &datas[mid];
            if mid_val.id <= target {
                ans = mid;
                lo = mid + 1;
            } else {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
        }
        ans
    }
}

impl ReStream {
    pub fn new(name: Bytes) -> Self {
        Self {
            _name: name,
            stream: RwLock::new(Vec::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.stream.read().unwrap().len()
    }

    pub fn add(&self, id: Bytes, bytes_vec: Vec<Bytes>) -> Result<Bytes> {
        let mut s = self.stream.write().unwrap();
        let last_id = s.last().map(|e| e.id).unwrap_or_else(|| EntryID::new(0, 0));
        let cur_id = last_id.next_id(id)?;
        s.push(StreamEntry::new(cur_id, bytes_vec));
        Ok(cur_id.to_bytes())
    }

    pub fn xrange(&self, start: Bytes, end: Bytes) -> Result<Vec<StreamEntry>> {
        let start = EntryID::from(start, 0)?;
        let end = EntryID::from(end, i64::MAX)?;
        let len = self.len();
        let start_idx = self.lower_bound(start);
        if start_idx == len {
            return Ok(Vec::new());
        }
        let end_idx = self.high_bound(end);
        if end_idx == len || start_idx > end_idx {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(end_idx - start_idx + 1);
        let stream = self.stream.read().unwrap();
        for i in start_idx..=end_idx {
            results.push(stream[i].clone());
        }
        Ok(results)
    }
}
