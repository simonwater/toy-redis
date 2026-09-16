use super::*;

/// binary search
pub(super) fn lower_bound(datas: &[StreamEntry], target: EntryID, include: bool) -> usize {
    let mut ans = datas.len();
    if datas.is_empty() {
        return ans;
    }
    let mut lo = 0;
    let mut hi = datas.len() - 1;
    while lo <= hi {
        let mid = lo + ((hi - lo) >> 1);
        let mid_val = &datas[mid];
        if include && mid_val.id == target || mid_val.id > target {
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

pub(super) fn high_bound(datas: &[StreamEntry], target: EntryID, include: bool) -> usize {
    let mut ans = datas.len();
    if datas.is_empty() {
        return ans;
    }
    let mut lo = 0;
    let mut hi = datas.len() - 1;
    while lo <= hi {
        let mid = lo + ((hi - lo) >> 1);
        let mid_val = &datas[mid];
        if include && mid_val.id == target || mid_val.id < target {
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
