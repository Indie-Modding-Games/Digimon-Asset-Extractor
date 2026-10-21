use crate::error::{DwExtractError, Result};

#[derive(Clone)]
pub struct ByteReader<'a> {
    data: &'a [u8],
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn u8(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    pub fn u16(&self, offset: usize) -> Option<u16> {
        if offset + 2 <= self.data.len() {
            Some(u16::from_le_bytes([self.data[offset], self.data[offset + 1]]))
        } else {
            None
        }
    }

    pub fn u32(&self, offset: usize) -> Option<u32> {
        if offset + 4 <= self.data.len() {
            Some(u32::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]))
        } else {
            None
        }
    }

    pub fn chars(&self, offset: usize, len: usize) -> Option<&str> {
        if offset + len <= self.data.len() {
            std::str::from_utf8(&self.data[offset..offset + len]).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_reader() {
        let data = b"hello world";
        let reader = ByteReader::new(data);
        assert_eq!(reader.u8(0), Some(b'h'));
        assert_eq!(reader.u16(0), Some(u16::from_le_bytes([b'h', b'e'])));
        assert_eq!(reader.u32(0), Some(u32::from_le_bytes([b'h', b'e', b'l', b'l'])));
        assert_eq!(reader.chars(0, 5), Some("hello"));
    }
}