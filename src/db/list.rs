use bytes::Bytes;
use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ReList {
    _name: Bytes,
    list: RwLock<VecDeque<Bytes>>,
    waiting_threads: Mutex<VecDeque<Thread>>, // 记录正在等待的线程句柄
}

impl ReList {
    pub fn new(name: Bytes) -> Self {
        Self {
            _name: name,
            list: RwLock::new(VecDeque::with_capacity(128)),
            waiting_threads: Mutex::new(VecDeque::with_capacity(16)),
        }
    }

    pub fn len(&self) -> i64 {
        self.list.read().unwrap().len() as i64
    }

    pub fn push_front(&self, val_iter: Vec<Bytes>) -> i64 {
        self.push_inner(val_iter, true)
    }

    pub fn push_back(&self, val_iter: Vec<Bytes>) -> i64 {
        self.push_inner(val_iter, false)
    }

    pub fn pop_front(&self, cnt: i64) -> Option<Vec<Bytes>> {
        self.pop_inner(cnt, true)
    }

    pub fn pop_back(&self, cnt: i64) -> Option<Vec<Bytes>> {
        self.pop_inner(cnt, false)
    }

    pub fn block_pop_front(&self, timeout_in_sec: f64) -> Option<Bytes> {
        self.block_pop_inner(timeout_in_sec, true)
    }

    pub fn block_pop_back(&self, timeout_in_sec: f64) -> Option<Bytes> {
        self.block_pop_inner(timeout_in_sec, false)
    }

    pub fn range(&self, mut start: i64, mut end: i64) -> Vec<Bytes> {
        let mut ans = Vec::new();
        let list_guard = self.list.read().unwrap();
        if list_guard.is_empty() {
            return ans;
        }
        if start < 0 {
            start = 0.max(list_guard.len() as i64 + start);
        }
        if end < 0 {
            end = 0.max(list_guard.len() as i64 + end);
        }
        let start = start as usize;
        let mut end = end as usize;
        end = end.min(list_guard.len() - 1);
        if start > end {
            return ans;
        }
        for i in start..=end {
            ans.push(list_guard[i].clone());
        }
        ans
    }
}

impl ReList {
    fn push_inner(&self, val_iter: Vec<Bytes>, is_front: bool) -> i64 {
        let len = {
            let mut list_guard = self.list.write().unwrap();
            for value in val_iter {
                if is_front {
                    list_guard.push_front(value);
                } else {
                    list_guard.push_back(value);
                }
            }
            list_guard.len()
        };

        {
            // 唤醒等待的线程
            let mut threads_guard = self.waiting_threads.lock().unwrap();
            for _ in 0..len {
                if let Some(thread_handle) = threads_guard.pop_front() {
                    thread_handle.unpark();
                } else {
                    // 没有等待线程，结束
                    break;
                }
            }
        }

        len as i64
    }

    fn pop_inner(&self, mut cnt: i64, is_front: bool) -> Option<Vec<Bytes>> {
        let mut list_guard = self.list.write().unwrap();
        cnt = cnt.min(list_guard.len() as i64);
        let mut ans = Vec::new();
        for _ in 0..cnt {
            let val = if is_front {
                list_guard.pop_front().unwrap()
            } else {
                list_guard.pop_back().unwrap()
            };
            ans.push(val);
        }
        if ans.len() > 0 {
            return Some(ans);
        }
        return None;
    }

    // 线程被唤醒，但竞争不过别的线程，重新进入等待时，超时时间需要减去计时开始到现在所过去的时间
    fn block_pop_inner(&self, mut timeout_in_sec: f64, is_front: bool) -> Option<Bytes> {
        if timeout_in_sec <= 0.0 || timeout_in_sec > 3600.0 {
            timeout_in_sec = 3600.0;
        }
        let start_time = Instant::now(); // 开始计时间
        let total_dur = Duration::from_secs_f64(timeout_in_sec);

        loop {
            if let Some(value) = self.try_pop(is_front) {
                return Some(value);
            }

            let pasted_dur = Instant::now() - start_time;
            if pasted_dur > total_dur {
                return None;
            }

            // 先注册再二次检查，关闭"检查失败到注册完成"之间的竞态窗口。
            // 若push发生在注册之前，push方在waiting_threads中
            // 看不到本线程，不会发起unpark，本线程将睡到超时。
            self.waiting_threads
                .lock()
                .unwrap()
                .push_back(thread::current());

            if let Some(value) = self.try_pop(is_front) {
                // 已拿到数据，从等待队列移除自身，避免推入方unpark一个不再等待的线程
                self.unregister_current();
                return Some(value);
            }

            thread::park_timeout(total_dur - pasted_dur); // 等待被唤起
            // 无论被唤醒还是超时都移除自身句柄，避免过期句柄消耗后续push的通知名额
            self.unregister_current();
        }
    }

    fn try_pop(&self, is_front: bool) -> Option<Bytes> {
        let mut list_guard = self.list.write().unwrap();
        if is_front {
            list_guard.pop_front()
        } else {
            list_guard.pop_back()
        }
    }

    fn unregister_current(&self) {
        let cur_id = thread::current().id();
        let mut threads_guard = self.waiting_threads.lock().unwrap();
        threads_guard.retain(|t| t.id() != cur_id);
    }
}
