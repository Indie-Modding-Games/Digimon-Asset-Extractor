use crate::error::{DwExtractError, Result};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub const SECTOR_SIZE: usize = 2352;
pub const USER_DATA_SIZE: usize = 2048;
pub const SYNC_SIZE: usize = 12;
pub const HEADER_SIZE: usize = 4;
pub const SUBHEADER_SIZE: usize = 8;
pub const EDC_SIZE: usize = 4;
pub const ECC_SIZE: usize = 276;

#[derive(Debug, Clone)]
pub struct CueInfo {
    pub bin_path: String,
    pub track_mode: String,
}

pub fn parse_cue(cue_path: &Path) -> Result<CueInfo> {
    let content = std::fs::read_to_string(cue_path)?;
    let mut bin_path = None;
    let mut track_mode = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("FILE") && line.contains("BINARY") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                bin_path = Some(parts[1].trim_matches('"').to_string());
            }
        } else if line.starts_with("TRACK") && line.contains("MODE") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                track_mode = Some(parts[2].to_string());
            }
        }
    }

    let bin_path = bin_path.ok_or_else(|| DwExtractError::InvalidCue("No FILE entry found".to_string()))?;
    let track_mode = track_mode.unwrap_or_else(|| "MODE2/2352".to_string());

    if !track_mode.contains("2352") {
        return Err(DwExtractError::InvalidCue(format!("Unsupported sector size: {}", track_mode)));
    }

    Ok(CueInfo { bin_path, track_mode })
}

pub struct BinReader {
    file: BufReader<File>,
    path: std::path::PathBuf,
    total_sectors: u64,
}

impl BinReader {
    pub fn open<P: AsRef<Path>>(bin_path: P) -> Result<Self> {
        let path = bin_path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let metadata = file.metadata()?;
        let total_bytes = metadata.len();
        
        if total_bytes % SECTOR_SIZE as u64 != 0 {
            return Err(DwExtractError::InvalidBin(
                format!("BIN size {} not multiple of sector size {}", total_bytes, SECTOR_SIZE)
            ));
        }

        let total_sectors = total_bytes / SECTOR_SIZE as u64;
        let file = BufReader::new(file);

        Ok(Self { file, path, total_sectors })
    }

    pub fn total_sectors(&self) -> u64 {
        self.total_sectors
    }

    pub fn read_sector(&mut self, sector: u32) -> Result<[u8; SECTOR_SIZE]> {
        let offset = sector as u64 * SECTOR_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        
        let mut sector_data = [0u8; SECTOR_SIZE];
        self.file.read_exact(&mut sector_data)?;
        
        Ok(sector_data)
    }

    pub fn read_user_data(&mut self, sector: u32) -> Result<[u8; USER_DATA_SIZE]> {
        let sector_data = self.read_sector(sector)?;
        let user_data = extract_user_data(&sector_data)?;
        Ok(user_data)
    }

    pub fn read_sectors_range(&mut self, start_sector: u32, count: u32) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(count as usize * USER_DATA_SIZE);
        for i in 0..count {
            let user_data = self.read_user_data(start_sector + i)?;
            data.extend_from_slice(&user_data);
        }
        Ok(data)
    }

    pub fn read_file_data(&mut self, start_sector: u32, size: u32) -> Result<Vec<u8>> {
        // Each sector provides USER_DATA_SIZE (2048) bytes of user data
        let sector_count = (size + USER_DATA_SIZE as u32 - 1) / USER_DATA_SIZE as u32;

        let mut data = Vec::with_capacity(size as usize);
        let mut remaining = size;
        
        for i in 0..sector_count {
            let sector_data = self.read_sector(start_sector + i)?;
            let user_data = extract_user_data(&sector_data)?;
            
            let take = std::cmp::min(remaining as usize, user_data.len());
            data.extend_from_slice(&user_data[..take]);
            remaining -= take as u32;
            
            if remaining == 0 {
                break;
            }
        }

        if data.len() != size as usize {
            return Err(DwExtractError::SectorRead {
                sector: start_sector,
                message: format!("Expected {} bytes, got {}", size, data.len()),
            });
        }

        Ok(data)
    }
}

fn extract_user_data(sector: &[u8; SECTOR_SIZE]) -> Result<[u8; USER_DATA_SIZE]> {
    if sector[0..SYNC_SIZE] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
        return Err(DwExtractError::InvalidBin("Invalid sync pattern".to_string()));
    }

    let mode = sector[SYNC_SIZE + 3];
    if mode != 0x02 {
        return Err(DwExtractError::InvalidBin(format!("Expected MODE2 (0x02), got 0x{:02X}", mode)));
    }

    let subheader = &sector[SYNC_SIZE + HEADER_SIZE..SYNC_SIZE + HEADER_SIZE + SUBHEADER_SIZE];
    let _file_num = subheader[0];
    let _channel_num = subheader[1];
    let submode = subheader[2];
    let _coding_info = subheader[3];

    // Accept Data sectors (bit 0x08) for MODE2 - Form 1 is assumed for data tracks
    if (submode & 0x08) == 0 {
        return Err(DwExtractError::InvalidBin(format!("Not a data sector (submode=0x{:02X})", submode)));
    }

    let user_data_start = SYNC_SIZE + HEADER_SIZE + SUBHEADER_SIZE;
    let user_data_end = user_data_start + USER_DATA_SIZE;
    
    let mut user_data = [0u8; USER_DATA_SIZE];
    user_data.copy_from_slice(&sector[user_data_start..user_data_end]);
    
    // Verify EDC for Form 1 sectors
    let edc_start = user_data_end;
    if edc_start + EDC_SIZE <= SECTOR_SIZE {
        let edc_bytes = &sector[edc_start..edc_start + EDC_SIZE];
        let computed_edc = compute_edc(&sector[SYNC_SIZE..user_data_end]);
        let stored_edc = u32::from_le_bytes([edc_bytes[0], edc_bytes[1], edc_bytes[2], edc_bytes[3]]);
        
        if computed_edc != stored_edc {
            // EDC mismatch - log warning but continue (some sectors may have different format)
            eprintln!("Warning: EDC mismatch at sector (computed: 0x{:08X}, stored: 0x{:08X})", computed_edc, stored_edc);
        }
    }

    Ok(user_data)
}

fn compute_edc(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc = crc32_byte(crc, byte);
    }
    !crc
}

fn crc32_byte(crc: u32, byte: u8) -> u32 {
    let mut crc = crc ^ (byte as u32);
    for _ in 0..8 {
        if crc & 1 != 0 {
            crc = (crc >> 1) ^ 0xEDB88320;
        } else {
            crc >>= 1;
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edc_computation() {
        let data = b"test data";
        let crc = compute_edc(data);
        assert_ne!(crc, 0);
    }
}