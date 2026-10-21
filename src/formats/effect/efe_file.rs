use crate::error::{DwExtractError, Result};
use std::io::{Read, Seek, SeekFrom};

pub const EFE_SLOT_SIZE: usize = 0x5000;
pub const EFE_HEADER_SIZE: usize = 0x34;
pub const EFE_EFFECT_ID_BASE: i32 = 0x100;
pub const EFE_FILL: u8 = 0xCD;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EfeHeader {
    pub start_script: i32,
    pub init_script: i32,
    pub uv_anims: i32,
    pub uv_anims_end: i32,
    pub sub_effects: i32,
    pub tmd_start: i32,
    pub tmd_end: i32,
    pub tim_start: i32,
    pub tim_end: i32,
    pub data_end: i32,
    pub spawn_count: i32,
    pub num_sub_effects: i32,
    pub effect_id: i32,
}

impl EfeHeader {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; EFE_HEADER_SIZE];
        reader.read_exact(&mut buf)?;
        Ok(Self {
            start_script: i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            init_script: i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            uv_anims: i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            uv_anims_end: i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            sub_effects: i32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]),
            tmd_start: i32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]),
            tmd_end: i32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]),
            tim_start: i32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]),
            tim_end: i32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]),
            data_end: i32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]),
            spawn_count: i32::from_le_bytes([buf[40], buf[41], buf[42], buf[43]]),
            num_sub_effects: i32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]),
            effect_id: i32::from_le_bytes([buf[48], buf[49], buf[50], buf[51]]),
        })
    }

    pub fn validate(&self, slot_size: usize) -> Result<()> {
        if self.data_end > slot_size as i32 {
            return Err(DwExtractError::InvalidFormat(format!(
                "EFE data_end ({}) exceeds slot size ({})", self.data_end, slot_size
            )));
        }
        if self.tmd_start >= self.tmd_end && self.tmd_start != 0 {
            return Err(DwExtractError::InvalidFormat("Invalid TMD range".into()));
        }
        if self.tim_start >= self.tim_end && self.tim_start != 0 {
            return Err(DwExtractError::InvalidFormat("Invalid TIM range".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EfeSlot {
    pub index: usize,
    pub header: EfeHeader,
    pub data: Vec<u8>,
    pub is_empty: bool,
}

impl EfeSlot {
    pub fn read<R: Read + Seek>(reader: &mut R, index: usize) -> Result<Self> {
        let slot_offset = index * EFE_SLOT_SIZE;
        reader.seek(SeekFrom::Start(slot_offset as u64))?;

        let header = EfeHeader::read(reader)?;
        
        let mut data = vec![0u8; EFE_SLOT_SIZE - EFE_HEADER_SIZE];
        reader.read_exact(&mut data)?;

        let is_empty = data.iter().all(|&b| b == EFE_FILL);

        Ok(Self {
            index,
            header,
            data,
            is_empty,
        })
    }

    pub fn payload(&self) -> &[u8] {
        let base = EFE_HEADER_SIZE;
        let end = self.header.data_end as usize;
        if end > base && end <= self.data.len() + EFE_HEADER_SIZE {
            &self.data[base - EFE_HEADER_SIZE..end - EFE_HEADER_SIZE]
        } else {
            &[]
        }
    }

pub fn tmd_data(&self) -> &[u8] {
        let base = EFE_HEADER_SIZE as i32;
        let start = (self.header.tmd_start - base) as usize;
        let end = (self.header.tmd_end - base) as usize;
        if start < self.data.len() && end <= self.data.len() && start < end {
            &self.data[start..end]
        } else {
            &[]
        }
    }
    
    pub fn tim_data(&self) -> &[u8] {
        let base = EFE_HEADER_SIZE as i32;
        let start = (self.header.tim_start - base) as usize;
        let end = (self.header.tim_end - base) as usize;
        if start < self.data.len() && end <= self.data.len() && start < end {
            &self.data[start..end]
        } else {
            &[]
        }
    }
    
    pub fn scripts_data(&self) -> &[u8] {
        let base = EFE_HEADER_SIZE as i32;
        let end = self.header.uv_anims - base;
        if end > 0 && end as usize <= self.data.len() {
            &self.data[..end as usize]
        } else {
            &[]
        }
    }
}

pub fn efe_slot_count(file_size: usize) -> Result<usize> {
    if file_size % EFE_SLOT_SIZE != 0 {
        return Err(DwExtractError::InvalidFormat(
            format!("EFE file size ({}) is not a multiple of slot size ({})", file_size, EFE_SLOT_SIZE)
        ));
    }
    Ok(file_size / EFE_SLOT_SIZE)
}

#[derive(Debug, Clone)]
pub struct EfeFile {
    pub slots: Vec<EfeSlot>,
}

impl EfeFile {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let file_size = reader.seek(SeekFrom::End(0))?;
        let count = efe_slot_count(file_size as usize)?;
        
        let mut slots = Vec::with_capacity(count);
        for i in 0..count {
            slots.push(EfeSlot::read(reader, i)?);
        }
        
        Ok(Self { slots })
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let count = efe_slot_count(data.len())?;
        let mut slots = Vec::with_capacity(count);
        
        for i in 0..count {
            let slot_offset = i * EFE_SLOT_SIZE;
            let slot_data = &data[slot_offset..slot_offset + EFE_SLOT_SIZE];
            
            let mut cursor = std::io::Cursor::new(slot_data);
            let header = EfeHeader::read(&mut cursor)?;
            
            let data = slot_data[EFE_HEADER_SIZE..].to_vec();
            let is_empty = data.iter().all(|&b| b == EFE_FILL);
            
            slots.push(EfeSlot {
                index: i,
                header,
                data,
                is_empty,
            });
        }
        
        Ok(Self { slots })
    }

    pub fn get_slot(&self, index: usize) -> Option<&EfeSlot> {
        self.slots.get(index)
    }

    pub fn used_slots(&self) -> Vec<&EfeSlot> {
        self.slots.iter().filter(|s| !s.is_empty).collect()
    }
}

pub fn parse_efe_file(data: &[u8]) -> Result<EfeFile> {
    EfeFile::from_bytes(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efe_constants() {
        assert_eq!(EFE_SLOT_SIZE, 0x5000);
        assert_eq!(EFE_HEADER_SIZE, 0x34);
        assert_eq!(EFE_EFFECT_ID_BASE, 0x100);
        assert_eq!(EFE_FILL, 0xCD);
    }

    #[test]
    fn test_efe_slot_count() {
        assert_eq!(efe_slot_count(0x5000).unwrap(), 1);
        assert_eq!(efe_slot_count(0x14000).unwrap(), 4);
        assert!(efe_slot_count(0x5001).is_err());
    }
}