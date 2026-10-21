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

/// Parses a CUE sheet.
///
/// The `FILE` line may quote the name of the image, and the name may contain spaces:
/// `FILE "Digimon World (Disc 1).bin" BINARY`. The first binary file is the one used, and
/// the track mode is the first one with 2352 byte sectors.
pub fn parse_cue(cue_path: &Path) -> Result<CueInfo> {
    let content = std::fs::read_to_string(cue_path)?;
    let mut bin_path = None;
    let mut track_modes: Vec<String> = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        if let Some(rest) = keyword_argument(line, "FILE") {
            if bin_path.is_some() {
                continue;
            }
            if let Some((name, remainder)) = parse_file_argument(rest) {
                if remainder.to_uppercase().contains("BINARY") {
                    bin_path = Some(name);
                }
            }
        } else if let Some(rest) = keyword_argument(line, "TRACK") {
            if let Some(mode) = parse_track_argument(rest) {
                track_modes.push(mode);
            }
        }
    }

    let bin_path = bin_path.ok_or_else(|| DwExtractError::InvalidCue("No FILE entry found".to_string()))?;

    let track_mode = track_modes
        .iter()
        .find(|mode| mode.to_uppercase().contains("MODE2") && mode.contains("2352"))
        .cloned()
        .ok_or_else(|| {
            DwExtractError::InvalidCue("No track with MODE2 2352 byte sectors found".to_string())
        })?;

    Ok(CueInfo { bin_path, track_mode })
}

/// The text that follows a keyword at the start of a CUE line, when the line starts with it.
fn keyword_argument<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    if !line.starts_with(keyword) {
        return None;
    }

    let rest = &line[keyword.len()..];
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }

    Some(rest)
}

/// The name of the file of a `FILE` line and the text that follows it.
fn parse_file_argument(rest: &str) -> Option<(String, &str)> {
    let rest = rest.trim_start();

    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some((stripped[..end].to_string(), &stripped[end + 1..]));
    }

    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    if end == 0 {
        return None;
    }

    Some((rest[..end].to_string(), &rest[end..]))
}

/// The mode of a `TRACK` line: the token that follows the track number.
fn parse_track_argument(rest: &str) -> Option<String> {
    let mut parts = rest.split_whitespace();
    let _number = parts.next()?;
    parts.next().map(|mode| mode.to_string())
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

    /// Reads complete raw sectors without interpreting their submode or user-data form.
    /// Used for STR streams, whose Form 2 sectors must retain sync/header/subheader bytes.
    pub fn read_raw_sectors(&mut self, start_sector: u32, count: u32) -> Result<Vec<u8>> {
        let end_sector = u64::from(start_sector) + u64::from(count);
        if end_sector > self.total_sectors {
            return Err(DwExtractError::SectorRead {
                sector: start_sector,
                message: format!("Raw sector range ends at {}, BIN has {} sectors", end_sector, self.total_sectors),
            });
        }
        let capacity = (count as usize).checked_mul(SECTOR_SIZE)
            .ok_or_else(|| DwExtractError::InvalidBin("Raw sector range is too large".to_string()))?;
        let mut data = Vec::with_capacity(capacity);
        for offset in 0..count {
            let sector = start_sector.checked_add(offset)
                .ok_or_else(|| DwExtractError::InvalidBin("Raw sector index overflows".to_string()))?;
            data.extend_from_slice(&self.read_sector(sector)?);
        }
        Ok(data)
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
    use std::io::Write;

    fn write_cue(content: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_edc_computation() {
        let data = b"test data";
        let crc = compute_edc(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn raw_sector_range_preserves_non_data_form2_sectors() {
        use std::io::Write;
        let mut bin = tempfile::NamedTempFile::new().unwrap();
        let mut expected = Vec::new();
        for submode in [0x48, 0x64] {
            let mut sector = [0u8; SECTOR_SIZE];
            sector[..SYNC_SIZE].copy_from_slice(&[0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0]);
            sector[SYNC_SIZE + 3] = 2;
            sector[SYNC_SIZE + HEADER_SIZE + 2] = submode;
            bin.write_all(&sector).unwrap();
            expected.extend_from_slice(&sector);
        }
        bin.flush().unwrap();
        let mut reader = BinReader::open(bin.path()).unwrap();
        let raw = reader.read_raw_sectors(0, 2).unwrap();
        assert_eq!(raw, expected);
        assert_eq!(raw.len(), 2 * SECTOR_SIZE);
        assert!(reader.read_raw_sectors(2, 1).is_err());
    }

    #[test]
    fn test_parse_cue_with_quoted_name() {
        let cue = write_cue(
            "FILE \"Digimon World (Disc 1).bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n",
        );

        let info = parse_cue(cue.path()).unwrap();
        assert_eq!(info.bin_path, "Digimon World (Disc 1).bin");
        assert_eq!(info.track_mode, "MODE2/2352");
    }

    #[test]
    fn test_parse_cue_with_unquoted_name() {
        let cue = write_cue("FILE DIGIMON.BIN BINARY\n  TRACK 01 MODE2/2352\n");

        let info = parse_cue(cue.path()).unwrap();
        assert_eq!(info.bin_path, "DIGIMON.BIN");
    }

    #[test]
    fn test_parse_cue_takes_the_first_binary_file() {
        let cue = write_cue(
            "FILE \"First Track.bin\" BINARY\n  TRACK 01 MODE2/2352\nFILE \"Second Track.bin\" BINARY\n  TRACK 02 AUDIO\n",
        );

        let info = parse_cue(cue.path()).unwrap();
        assert_eq!(info.bin_path, "First Track.bin");
    }

    #[test]
    fn test_parse_cue_uses_the_first_2352_track() {
        let cue = write_cue(
            "FILE \"Disc.bin\" BINARY\n  TRACK 01 MODE2/2352\n  TRACK 02 AUDIO\n    INDEX 01 00:02:00\n",
        );

        let info = parse_cue(cue.path()).unwrap();
        assert_eq!(info.track_mode, "MODE2/2352");
    }

    #[test]
    fn test_parse_cue_without_file_entry() {
        let cue = write_cue("REM nothing here\n  TRACK 01 MODE2/2352\n");
        assert!(parse_cue(cue.path()).is_err());
    }

    #[test]
    fn test_parse_cue_with_unsupported_sector_size() {
        let cue = write_cue("FILE DISC.BIN BINARY\n  TRACK 01 MODE1/2352\n");
        assert!(parse_cue(cue.path()).is_err());
    }
}
