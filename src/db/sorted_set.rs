use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    sync::RwLock,
};

use bytes::Bytes;

#[derive(Debug, Clone)]
struct ZSetEntry {
    pub score: f64,
    pub member: Bytes,
}

impl ZSetEntry {
    pub fn new(score: f64, member: Bytes) -> Self {
        Self { score, member }
    }
}

impl PartialEq for ZSetEntry {
    fn eq(&self, other: &ZSetEntry) -> bool {
        self.score == other.score && self.member == other.member
    }
}

impl Eq for ZSetEntry {}

impl PartialOrd for ZSetEntry {
    fn partial_cmp(&self, other: &ZSetEntry) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ZSetEntry {
    fn cmp(&self, other: &ZSetEntry) -> Ordering {
        let ord = self.score.partial_cmp(&other.score).unwrap(); // 不会存在NaN
        if ord != Ordering::Equal {
            return ord;
        }
        self.member.cmp(&other.member)
    }
}

#[derive(Debug)]
struct ZSetInner {
    score_map: HashMap<Bytes, f64>,
    order_set: BTreeSet<ZSetEntry>,
}

impl ZSetInner {
    fn new() -> Self {
        Self {
            score_map: HashMap::with_capacity(16),
            order_set: BTreeSet::new(),
        }
    }

    fn add(&mut self, score: f64, member: Bytes) -> bool {
        // 默认行为，插入或更新，更新不计入结果
        let result = if let Some(old_score) = self.score_map.insert(member.clone(), score) {
            if old_score == score {
                return false;
            }

            self.order_set
                .remove(&ZSetEntry::new(old_score, member.clone()));
            false
        } else {
            true
        };
        self.order_set.insert(ZSetEntry::new(score, member));
        result
    }

    fn rank(&self, member: Bytes) -> Option<i64> {
        let score = self.score_map.get(&member).copied()?;
        let entry = ZSetEntry::new(score, member);
        let r = self.order_set.range(..entry).count() as i64;
        return Some(r);
    }

    fn range(&self, mut start: i64, mut end: i64) -> Vec<Bytes> {
        if start < 0 {
            start = 0.max(self.order_set.len() as i64 + start);
        }
        if end < 0 {
            end = 0.max(self.order_set.len() as i64 + end);
        }
        let start = start as usize;
        let mut end = end as usize;
        end = end.min(self.order_set.len() - 1);
        if start > end {
            return Vec::new();
        }
        let count = end - start + 1;
        self.order_set
            .iter()
            .skip(start)
            .take(count)
            .map(|entry| entry.member.clone())
            .collect()
    }

    fn len(&self) -> i64 {
        self.order_set.len() as i64
    }

    fn score(&self, member: Bytes) -> Option<f64> {
        self.score_map.get(&member).copied()
    }
}

#[derive(Debug)]
pub struct SortedSet {
    zset: RwLock<ZSetInner>,
}

impl SortedSet {
    pub fn new() -> Self {
        Self {
            zset: RwLock::new(ZSetInner::new()),
        }
    }

    pub fn add(&self, pairs: Vec<(f64, Bytes)>) -> i64 {
        let mut set = self.zset.write().unwrap();
        let mut cnt = 0;
        for (score, member) in pairs {
            if set.add(score, member) {
                cnt += 1;
            }
        }
        cnt
    }

    pub fn rank(&self, member: Bytes) -> Option<i64> {
        let set = self.zset.read().unwrap();
        set.rank(member)
    }

    pub fn range(&self, start: i64, end: i64) -> Vec<Bytes> {
        let set = self.zset.read().unwrap();
        set.range(start, end)
    }

    pub fn len(&self) -> i64 {
        let set = self.zset.read().unwrap();
        set.len()
    }

    pub fn score(&self, member: Bytes) -> Option<f64> {
        let set = self.zset.read().unwrap();
        set.score(member)
    }
}
