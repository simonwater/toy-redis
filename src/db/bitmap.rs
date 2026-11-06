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
