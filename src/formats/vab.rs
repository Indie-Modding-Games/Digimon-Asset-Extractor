use crate::error::{DwExtractError, Result};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct VabHeader {
    pub file_size: u32,
    pub version: u32,
    pub header_size: u32,
    pub magic: [u8; 4],
    pub unk1: u32,
    pub unk2: u32,
    pub program_count: u16,
    pub attribute_count: u16,
    pub mvol: u16,
    pub pan: u16,
    pub attr_offset: u32,
    pub prog_offset: u32,
    pub vag_offset: u32,
    pub seq_offset: u32,
}

#[derive(Debug, Clone)]
pub struct VabProgram {
    pub tone_count: u8,
    pub priority: u8,
    pub mode: u8,
    pub mvol: u8,
    pub pan: u8,
    pub attr_index: u16,
}

#[derive(Debug, Clone)]
pub struct VabAttribute {
    pub priority: u8,
    pub mode: u8,
    pub adsr1: u16,
    pub adsr2: u16,
    pub vibrato_depth: u8,
    pub vibrato_rate: u8,
    pub portamento_time: u8,
    pub portamento_mode: u8,
    pub unk: [u8; 4],
    pub vag_index: u16,
    pub pitch_lfo: u16,
    pub unk2: [u8; 4],
}

#[derive(Debug, Clone)]
pub struct VabVagEntry {
    pub sample_rate: u32,
    pub loop_start: u32,
    pub loop_end: u32,
    pub data_offset: u32,
    pub data_size: u32,
}

#[derive(Debug, Clone)]
pub struct VabFile {
    pub header: VabHeader,
    pub programs: Vec<VabProgram>,
    pub attributes: Vec<VabAttribute>,
    pub vag_entries: Vec<VabVagEntry>,
    pub vag_data: Vec<u8>,
}

impl VabFile {
    /// Parse all VAB structures in a file that may contain multiple concatenated VABs
    /// Extracts raw VAB blocks between each "pBAV" (0x56414270) marker
    pub fn parse_all(data: &[u8]) -> Result<Vec<Self>> {
        let mut vabs = Vec::new();
        let mut offset = 0;
        
        while offset + 4 < data.len() {
            // Search for VAB magic "pBAV" (0x56414270 little-endian) at current offset
            if offset + 4 > data.len() {
                break;
            }
            
            let magic_u32 = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            
            // Check for VAB magic "pBAV" (0x56414270 little-endian)
            if magic_u32 != 0x56414270 {
                // Search for next "pBAV" magic
                let next_magic = data[offset..].windows(4).skip(1).position(|w| {
                    u32::from_le_bytes([w[0], w[1], w[2], w[3]]) == 0x56414270
                });
                if let Some(pos) = next_magic {
                    offset += pos + 1;
                } else {
                    break;
                }
                continue;
            }
            
            // Find the next VAB magic to determine block size
            let next_magic = data[offset + 4..].windows(4).position(|w| {
                u32::from_le_bytes([w[0], w[1], w[2], w[3]]) == 0x56414270
            });
            
            let block_end = if let Some(pos) = next_magic {
                offset + 4 + pos
            } else {
                data.len()
            };
            
            let block_size = block_end - offset;
            if block_size < 0x20 {
                offset += 4;
                continue;
            }
            
            // Extract raw VAB block
            let block_data = &data[offset..block_end];
            
            // Create a minimal VabFile with raw data for external processing
            let vab = Self {
                header: VabHeader {
                    file_size: block_size as u32,
                    version: 0,
                    header_size: 0,
                    magic: *b"pBAV",
                    unk1: 0,
                    unk2: 0,
                    program_count: 0,
                    attribute_count: 0,
                    mvol: 0,
                    pan: 0,
                    attr_offset: 0,
                    prog_offset: 0,
                    vag_offset: 0,
                    seq_offset: 0,
                },
                programs: Vec::new(),
                attributes: Vec::new(),
                vag_entries: Vec::new(),
                vag_data: block_data.to_vec(),
            };
            
            vabs.push(vab);
            
            // Move to next block
            offset = block_end;
        }
        
        Ok(vabs)
    }
}