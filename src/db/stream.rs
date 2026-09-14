use anyhow::{Ok, Result, anyhow, bail};
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

    fn from(id: &Bytes, default_seq: i64) -> Result<Self> {
        let id_str = std::str::from_utf8(id)?;
        let mut ms_seq = id_str.split('-');
        let ms_str = ms_seq.next().unwrap();
        let ms: i64 = ms_str.parse()?;
        let seq = if let Some(seq_str) = ms_seq.next() {
            seq_str.parse()?
        } else {
            default_seq
        };
        Ok(Self { ms, seq })
    }

    fn next_id(&self, mut pat: Bytes) -> Result<Self> {
        if pat == "0-0" {
            bail!("ERR The ID specified in XADD must be greater than 0-0")
        }
        if pat == "*" {
            pat = Bytes::from("*-*");
        }

        let pat_str = std::str::from_utf8(&pat)?;
        let (ms_str, seq_str) = pat_str
            .split_once('-')
            .ok_or_else(|| anyhow!("ERR XADD missing entry ID or the entry ID format error"))?;
        let ms = if ms_str == "*" {
            Utc::now().timestamp_millis().max(self.ms)
        } else {
            ms_str.parse::<i64>()?
        };
        if ms < self.ms {
            bail!(
                "ERR The ID specified in XADD is equal or smaller than the target stream top item"
            );
        }

        let seq = if seq_str == "*" {
            if ms == self.ms {
                self.seq + 1
            } else if ms == 0 {
                1
            } else {
                0
            }
        } else {
            seq_str.parse::<i64>()?
        };
        if ms == self.ms && seq <= self.seq {
            bail!(
                "ERR The ID specified in XADD is equal or smaller than the target stream top item"
            );
        }
        return Ok(Self::new(ms, seq));
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
    fn lower_bound(&self, target: EntryID, include: bool) -> usize {
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
                if include || mid_val.id > target {
                    ans = mid;
                }
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

    fn high_bound(&self, target: EntryID, include: bool) -> usize {
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
                if include || mid_val.id < target {
                    ans = mid;
                }
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
        let len = self.len();
        let start_idx = if &start[..] == b"-" {
            0
        } else {
            let start = EntryID::from(&start, 0)?;
            self.lower_bound(start, true)
        };
        if start_idx == len {
            return Ok(Vec::new());
        }

        let end_idx = if &end[..] == b"+" {
            len - 1
        } else {
            let end = EntryID::from(&end, i64::MAX)?;
            self.high_bound(end, true)
        };
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

    pub fn xread(&self, id: &Bytes) -> Result<Vec<StreamEntry>> {
        let n = self.len();
        let id = EntryID::from(id, 0)?;
        let idx = self.lower_bound(id, false);
        if idx == n {
            return Ok(Vec::new());
        }
        let mut results = Vec::with_capacity(n - idx);
        let stream = self.stream.read().unwrap();
        for i in idx..n {
            results.push(stream[i].clone());
        }
        Ok(results)
    }
}
