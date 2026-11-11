use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut};

pub(super) fn set_bit(bytes: &mut BytesMut, offset: usize, bit_val: u8) -> Result<u8> {
    if bit_val != 0 && bit_val != 1 {
        bail!("ERR bit value only can be 1 or 0.")
    }
    let len = bytes.len();
    let bytes_index = offset / 8;
    let bit_offset = 7 - (offset % 8); // 从左到右转为从右到左

    if bytes_index >= len {
        // 长度不够
        bytes.resize(bytes_index + 1, 0);
    }
    let current_byte = bytes[bytes_index];
    let old_bit = (current_byte >> bit_offset) & 1;

    bytes[bytes_index] = if bit_val == 1 {
        current_byte | (1 << bit_offset)
    } else {
        current_byte & !(1 << bit_offset)
    };

    Ok(old_bit)
}

pub(super) fn get_bit(bytes: Bytes, offset: usize) -> u8 {
    let len = bytes.len();
    let bytes_index = offset / 8;
    if bytes_index >= len {
        return 0;
    }
    let bit_offset = 7 - (offset % 8);
    let current_byte = bytes[bytes_index];
    (current_byte >> bit_offset) & 1
}

pub(super) fn bit_count(bytes: Bytes, mut start: i64, mut end: i64) -> i64 {
    if bytes.is_empty() {
        return 0;
    }
    if start < 0 {
        start = 0.max(bytes.len() as i64 + start);
    }
    if end < 0 {
        end = 0.max(bytes.len() as i64 + end);
    }
    let start = start as usize;
    let mut end = end as usize;
    end = end.min(bytes.len() - 1);
    if start > end {
        return 0;
    }
    bytes[start..=end]
        .iter()
        .map(|b| b.count_ones())
        .sum::<u32>() as i64
}

pub(super) fn and(bytes1: Bytes, bytes2: Bytes) -> Bytes {
    let mut bytes_res = BytesMut::zeroed(bytes1.len().max(bytes2.len()));
    for i in 0..bytes1.len().min(bytes2.len()) {
        bytes_res[i] = bytes1[i] & bytes2[i];
    }

    bytes_res.freeze()
}

pub(super) fn or(bytes1: Bytes, bytes2: Bytes) -> Bytes {
    let mut bytes_res = if bytes1.len() > bytes2.len() {
        BytesMut::from(bytes1.clone())
    } else {
        BytesMut::from(bytes2.clone())
    };
    for i in 0..bytes1.len().min(bytes2.len()) {
        bytes_res[i] = bytes1[i] | bytes2[i];
    }

    bytes_res.freeze()
}
