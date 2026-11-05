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
}
