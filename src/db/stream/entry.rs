use anyhow::{Ok, Result, anyhow, bail};
use bytes::Bytes;
use chrono::Utc;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct EntryID {
    pub(super) ms: i64,
    pub(super) seq: i64,
}

impl EntryID {
    pub(super) fn new(ms: i64, seq: i64) -> Self {
        Self { ms, seq }
    }

    pub(super) fn from(id: &Bytes, default_seq: i64) -> Result<Self> {
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

    pub(super) fn next_id(&self, mut pat: Bytes) -> Result<Self> {
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
    pub(super) id: EntryID,
    pub(super) value: Vec<Bytes>,
}

impl StreamEntry {
    pub(super) fn new(id: EntryID, value: Vec<Bytes>) -> Self {
        Self { id, value }
    }

    pub(crate) fn get_id(&self) -> EntryID {
        self.id
    }

    pub(crate) fn to_value(self) -> Vec<Bytes> {
        self.value
    }
}
