use crate::error::{DwExtractError, Result};
use crate::formats::tim::TimHeader;
use binrw::{BinRead, BinReaderExt};
use std::io::Cursor;

#[derive(Debug, Clone, BinRead)]
#[br(little)]
pub struct TmdHeader {
    pub id: u32,
    pub flags: u32,
    pub n_obj: u32,
}

#[derive(Debug, Clone, BinRead)]
#[br(little)]
pub struct TmdObject {
    pub vert_offset: u32,
    pub vert_count: u32,
    pub normal_offset: u32,
    pub normal_count: u32,
    pub prim_offset: u32,
    pub prim_count: u32,
    pub scale: [i16; 3],
    pub _pad: u32,
    pub center: [i16; 3],
    pub _pad2: u32,
}

#[derive(Debug, Clone)]
pub struct MmdInfo {
    pub tmd_header: TmdHeader,
    pub objects: Vec<TmdObject>,
    pub texture_info: Option<TimInfo>,
    pub total_size: usize,
}

#[derive(Debug, Clone)]
pub struct TimInfo {
    pub bpp: u8,
    pub has_clut: bool,
    pub width: u16,
    pub height: u16,
    pub data_size: usize,
}

impl MmdInfo {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(DwExtractError::InvalidMmd("Data too small for MMD header".to_string()));
        }

        let mut cursor = Cursor::new(data);
        let tmd_header: TmdHeader = BinReaderExt::read_le(&mut cursor)
            .map_err(|e| DwExtractError::InvalidMmd(format!("Failed to read TMD header: {}", e)))?;
        
        if tmd_header.id != 0x3D4D54 {
            return Err(DwExtractError::InvalidMmd(format!("Invalid TMD ID: 0x{:08X}, expected 0x3D4D54 ('TMD=')", tmd_header.id)));
        }

        let mut objects = Vec::with_capacity(tmd_header.n_obj as usize);
        for _ in 0..tmd_header.n_obj {
            let obj: TmdObject = BinReaderExt::read_le(&mut cursor)
                .map_err(|e| DwExtractError::InvalidMmd(format!("Failed to read TMD object: {}", e)))?;
            objects.push(obj);
        }

        let tmd_data_end = cursor.position() as usize;
        
        let texture_info = if data.len() > tmd_data_end {
            let texture_data = &data[tmd_data_end..];
            if texture_data.len() >= 8 {
                TimHeader::parse(texture_data).ok().map(|t| TimInfo {
                    bpp: t.bpp,
                    has_clut: t.has_clut,
                    width: t.width,
                    height: t.height,
                    data_size: texture_data.len(),
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            tmd_header,
            objects,
            texture_info,
            total_size: data.len(),
        })
    }

    pub fn validate(data: &[u8]) -> Result<Self> {
        let info = Self::parse(data)?;
        
        if info.tmd_header.n_obj == 0 {
            return Err(DwExtractError::InvalidMmd("No objects in MMD".to_string()));
        }
        
        if info.objects.is_empty() {
            return Err(DwExtractError::InvalidMmd("Failed to parse objects".to_string()));
        }

        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmd_tmd_header() {
        let tmd_data = vec![
            // TMD Header (12 bytes)
            0x54, 0x4D, 0x3D, 0x00,  // ID = 'TMD=' (little-endian: 0x003D4D54)
            0x00, 0x00, 0x00, 0x00,  // Flags
            0x01, 0x00, 0x00, 0x00,  // n_obj = 1
            // TMD Object (44 bytes)
            0x00, 0x00, 0x00, 0x00,  // vert_offset
            0x08, 0x00, 0x00, 0x00,  // vert_count = 8
            0x00, 0x00, 0x00, 0x00,  // normal_offset
            0x08, 0x00, 0x00, 0x00,  // normal_count = 8
            0x00, 0x00, 0x00, 0x00,  // prim_offset
            0x04, 0x00, 0x00, 0x00,  // prim_count = 4
            0x01, 0x00, 0x02, 0x00, 0x03, 0x00,  // scale: [1, 2, 3] (3 * i16 = 6 bytes)
            0x00, 0x00, 0x00, 0x00,  // _pad
            0x04, 0x00, 0x05, 0x00, 0x06, 0x00,  // center: [4, 5, 6] (3 * i16 = 6 bytes)
            0x00, 0x00, 0x00, 0x00,  // _pad2
        ];
        
        let info = MmdInfo::validate(&tmd_data).unwrap();
        assert_eq!(info.tmd_header.n_obj, 1);
        assert_eq!(info.objects.len(), 1);
        assert_eq!(info.objects[0].vert_count, 8);
    }
}