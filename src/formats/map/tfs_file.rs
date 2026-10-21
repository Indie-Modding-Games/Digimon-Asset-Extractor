use crate::error::{DwExtractError, Result};
use std::io::{Read, Seek, SeekFrom};

fn read_i16<R: Read>(reader: &mut R) -> Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_u16<R: Read>(reader: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TfsHeader {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub cell_size: u32,
    pub flags: u32,
}

impl TfsHeader {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let version = read_u32(reader)?;
        let width = read_u32(reader)?;
        let height = read_u32(reader)?;
        let cell_size = read_u32(reader)?;
        let flags = read_u32(reader)?;
        
        Ok(Self { version, width, height, cell_size, flags })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TfsCell {
    pub ground_height: i16,
    pub water_height: i16,
    pub attr: u8,
    pub trigger: u8,
    pub texture: u16,
    pub unknown: [u8; 8],
}

impl TfsCell {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let ground_height = read_i16(reader)?;
        let water_height = read_i16(reader)?;
        let attr = read_u8(reader)?;
        let trigger = read_u8(reader)?;
        let texture = read_u16(reader)?;
        
        let mut unknown = [0u8; 8];
        reader.read_exact(&mut unknown)?;
        
        Ok(Self {
            ground_height, water_height, attr, trigger, texture, unknown,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TfsFile {
    pub header: TfsHeader,
    pub cells: Vec<TfsCell>,
}

impl TfsFile {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let header = TfsHeader::read(reader)?;
        
        let cell_count = (header.width * header.height) as usize;
        let mut cells = Vec::with_capacity(cell_count);
        
        for _ in 0..cell_count {
            cells.push(TfsCell::read(reader)?);
        }
        
        Ok(Self { header, cells })
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = std::io::Cursor::new(data);
        Self::read(&mut cursor)
    }
    
    pub fn get_cell(&self, x: u32, y: u32) -> Option<&TfsCell> {
        if x >= self.header.width || y >= self.header.height {
            return None;
        }
        let idx = (y * self.header.width + x) as usize;
        self.cells.get(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfs_header() {
        let header = TfsHeader {
            version: 1,
            width: 100,
            height: 100,
            cell_size: 16,
            flags: 0,
        };
        assert_eq!(header.width, 100);
        assert_eq!(header.height, 100);
    }
}