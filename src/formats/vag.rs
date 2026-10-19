use crate::error::{DwExtractError, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use hound::{WavSpec, WavWriter, SampleFormat};
use std::io::{Cursor, Read, Seek, SeekFrom};

#[derive(Debug, Clone)]
pub struct VagHeader {
    pub id: [u8; 4],
    pub version: u32,
    pub reserved: [u32; 2],
    pub channels: u16,
    pub sample_rate: u32,
    pub unk1: u16,
    pub unk2: u16,
    pub loop_start: u32,
    pub loop_end: u32,
    pub unk3: u32,
}

#[derive(Debug, Clone)]
pub struct VagStream {
    pub header: VagHeader,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct VhbFile {
    pub streams: Vec<VagStream>,
}

impl VhbFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(DwExtractError::InvalidBin("Data too small for VHB".to_string()));
        }

        let mut cursor = Cursor::new(data);
        
        // Check for VHB magic
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;
        if &magic != b"VHB\0" && &magic != b"Vhb\0" {
            // Might be a raw VAG file, try to parse as single stream
            cursor.seek(SeekFrom::Start(0))?;
            let stream = Self::parse_vag_stream(&mut cursor, data)?;
            return Ok(Self { streams: vec![stream] });
        }

        let _version = cursor.read_u32::<LittleEndian>()?;
        
        let mut streams = Vec::new();
        
        // Parse VAG streams until end of file
        while cursor.position() < data.len() as u64 {
            match Self::parse_vag_stream(&mut cursor, data) {
                Ok(stream) => streams.push(stream),
                Err(_) => break,
            }
        }

        Ok(Self { streams })
    }

    fn parse_vag_stream(cursor: &mut Cursor<&[u8]>, full_data: &[u8]) -> Result<VagStream> {
        let start_pos = cursor.position();
        
        // Check if we have enough data for VAG header (16 bytes minimum)
        if (full_data.len() as u64 - cursor.position()) < 16 {
            return Err(DwExtractError::InvalidBin("Incomplete VAG stream".to_string()));
        }

        let mut id = [0u8; 4];
        cursor.read_exact(&mut id)?;
        
        // Check for VAG magic
        if &id != b"VAGp" && &id != b"VAG\0" && &id != b"vag\0" {
            // Not a VAG stream, rewind
            cursor.seek(SeekFrom::Start(start_pos))?;
            return Err(DwExtractError::InvalidBin("No VAG stream found".to_string()));
        }

        let version = cursor.read_u32::<LittleEndian>()?;
        let mut reserved = [0u32; 2];
        reserved[0] = cursor.read_u32::<LittleEndian>()?;
        reserved[1] = cursor.read_u32::<LittleEndian>()?;
        
        let channels = cursor.read_u16::<LittleEndian>()?;
        let sample_rate = cursor.read_u32::<LittleEndian>()?;
        let unk1 = cursor.read_u16::<LittleEndian>()?;
        let unk2 = cursor.read_u16::<LittleEndian>()?;
        let loop_start = cursor.read_u32::<LittleEndian>()?;
        let loop_end = cursor.read_u32::<LittleEndian>()?;
        let unk3 = cursor.read_u32::<LittleEndian>()?;

        let header = VagHeader {
            id,
            version,
            reserved,
            channels,
            sample_rate,
            unk1,
            unk2,
            loop_start,
            loop_end,
            unk3,
        };

        // Read remaining data as ADPCM
        let remaining = full_data.len() - cursor.position() as usize;
        let mut adpcm_data = vec![0u8; remaining];
        cursor.read_exact(&mut adpcm_data)?;

        Ok(VagStream { header, data: adpcm_data })
    }

    pub fn to_wav_files(&self, output_dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
        std::fs::create_dir_all(output_dir)?;
        
        let mut output_files = Vec::new();
        
        for (i, stream) in self.streams.iter().enumerate() {
            let filename = output_dir.join(format!("stream_{:03}.wav", i));
            Self::convert_vag_to_wav(stream, &filename)?;
            output_files.push(filename);
        }
        
        Ok(output_files)
    }

    fn convert_vag_to_wav(stream: &VagStream, output_path: &std::path::Path) -> Result<()> {
        let header = &stream.header;
        let adpcm_data = &stream.data;

        let spec = WavSpec {
            channels: header.channels as u16,
            sample_rate: header.sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec)
            .map_err(|e| DwExtractError::InvalidBin(format!("Failed to create WAV writer: {}", e)))?;

        // Decode PSX ADPCM to PCM
        let pcm_data = Self::decode_adpcm(adpcm_data, header.channels as usize)?;
        
        for sample in pcm_data {
            writer.write_sample(sample)
                .map_err(|e| DwExtractError::InvalidBin(format!("Failed to write WAV sample: {}", e)))?;
        }

        writer.finalize()
            .map_err(|e| DwExtractError::InvalidBin(format!("Failed to finalize WAV: {}", e)))?;

        Ok(())
    }

    fn decode_adpcm(data: &[u8], channels: usize) -> Result<Vec<i16>> {
        // PSX ADPCM decoder
        // Each block is 16 bytes: 2 bytes header + 14 bytes data (28 nibbles = 28 samples)
        // Header: filter (4 bits), shift (4 bits)
        // Then 14 bytes = 28 nibbles for each channel

        let bytes_per_block = 16;
        let blocks_per_channel = data.len() / (bytes_per_block * channels);
        
        let mut output = Vec::new();
        output.reserve(data.len() * 2);

        // State for each channel
        let mut prev1 = vec![0i16; channels];
        let mut prev2 = vec![0i16; channels];

        for block_idx in 0..blocks_per_channel {
            for ch in 0..channels {
                let block_offset = (block_idx * channels + ch) * bytes_per_block;
                if block_offset + bytes_per_block > data.len() { break; }

                let block = &data[block_offset..block_offset + bytes_per_block];
                
                // Parse header
                let shift = block[0] & 0x0F;
                let filter = (block[0] >> 4) & 0x0F;
                
                // Filter coefficients
                let (k1, k2) = match filter {
                    0 => (0.0, 0.0),
                    1 => (0.9375, 0.0),
                    2 => (1.796875, -0.8125),
                    3 => (1.53125, -0.859375),
                    _ => (0.0, 0.0),
                };

                // Decode 28 nibbles (14 bytes)
                for byte_idx in 1..15 {
                    let byte = block[byte_idx];
                    let nibbles = [byte & 0x0F, byte >> 4];
                    
                    for &nibble in &nibbles {
                        let sample = Self::decode_nibble(nibble, shift, k1, k2, prev1[ch], prev2[ch]);
                        prev2[ch] = prev1[ch];
                        prev1[ch] = sample;
                        output.push(sample);
                    }
                }
            }
        }

        Ok(output)
    }

    fn decode_nibble(nibble: u8, shift: u8, k1: f64, k2: f64, prev1: i16, prev2: i16) -> i16 {
        // Sign extend 4-bit to 16-bit
        let mut sample = nibble as i16;
        if sample & 0x08 != 0 {
            sample = sample | -16; // Sign extend: 0xFFF0 as i16 = -16
        }
        
        // Apply shift
        sample <<= shift;
        
        // Apply filter
        let filtered = sample as f64 + k1 * prev1 as f64 + k2 * prev2 as f64;
        
        // Clamp to 16-bit range
        let clamped = filtered.clamp(-32768.0, 32767.0) as i16;
        
        clamped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_nibble() {
        // Test sign extension
        let result = VhbFile::decode_nibble(0x08, 0, 0.0, 0.0, 0, 0);
        assert_eq!(result, -8 << 0); // Sign extended
        
        let result = VhbFile::decode_nibble(0x07, 0, 0.0, 0.0, 0, 0);
        assert_eq!(result, 7);
    }
}