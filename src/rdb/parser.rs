use std::io::Read;

pub struct RdbParser<R> {
    _reader: R,
}

impl<R: Read> RdbParser<R> {
    pub fn new(_reader: R) -> Self {
        Self { _reader }
    }
}
