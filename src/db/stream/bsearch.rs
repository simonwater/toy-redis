use super::*;

/// binary search
pub(super) fn lower_bound(re_stream: &ReStream, target: EntryID, include: bool) -> usize {
    let datas = re_stream.stream.read().unwrap();
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

pub(super) fn high_bound(re_stream: &ReStream, target: EntryID, include: bool) -> usize {
    let datas = re_stream.stream.read().unwrap();
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
