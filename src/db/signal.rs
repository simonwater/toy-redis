use chrono::{DateTime, Utc};
use std::sync::{Condvar, Mutex};

#[derive(Debug)]
pub struct Signal {
    mutex: Mutex<bool>, // 状态
    condvar: Condvar,
}

impl Signal {
    pub(super) fn new() -> Self {
        Self {
            mutex: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    pub(super) fn wait_until(&self, deadline: DateTime<Utc>) -> bool {
        let mut notified = self.mutex.lock().unwrap();
        while !*notified {
            let now = Utc::now();
            if now >= deadline {
                return false;
            }

            let dur = deadline - now;
            let (result, _) = self
                .condvar
                .wait_timeout(notified, dur.to_std().unwrap())
                .unwrap();

            notified = result;
            if Utc::now() >= deadline {
                return false;
            }
        }
        // 被成功唤醒，重置通知标记以支持下一次循环（如果需要继续等）
        *notified = false;
        true
    }

    pub(super) fn notify(&self) {
        let mut notified = self.mutex.lock().unwrap();
        *notified = true;
        self.condvar.notify_one();
    }
}
