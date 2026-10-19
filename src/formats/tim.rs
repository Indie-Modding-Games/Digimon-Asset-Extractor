use crate::error::{DwExtractError, Result};
use binrw::{BinRead, BinReaderExt};
use image::{ImageBuffer, Rgb, Rgba};
use std::io::Cursor;

#[derive(Debug, Clone, BinRead)]
#[br(little)]
pub struct TimHeader {
    pub id: u32,
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct TimInfo {
    pub bpp: u8,
    pub has_clut: bool,
    pub width: u16,
    pub height: u16,
    pub clut_x: u16,
    pub clut_y: u16,
    pub clut_width: u16,
    pub clut_height: u16,
    pub pixel_data_size: usize,
    pub clut_data_size: usize,
}

#[derive(Debug, Clone)]
pub struct TimImage {
    pub info: TimInfo,
    pub clut: Option<Vec<u8>>,
    pub pixel_data: Vec<u8>,
}

impl TimHeader {
    pub fn parse(data: &[u8]) -> Result<TimInfo> {
        if data.len() < 8 {
            return Err(DwExtractError::InvalidTim("Data too small for TIM header".to_string()));
        }

        let mut cursor = Cursor::new(data);
        let header: TimHeader = BinReaderExt::read_le(&mut cursor)
            .map_err(|e| DwExtractError::InvalidTim(format!("Failed to read TIM header: {}", e)))?;
        
        if header.id != 0x10 {
            return Err(DwExtractError::InvalidTim(format!("Invalid TIM ID: 0x{:08X}, expected 0x10", header.id)));
        }

        let bpp = match header.flags & 0x03 {
            0 => 4,
            1 => 8,
            2 => 16,
            3 => 24,
            _ => unreachable!(),
        };

        let has_clut = (header.flags & 0x08) != 0;
        let _is_transparent = (header.flags & 0x10) != 0;

        let mut info = TimInfo {
            bpp,
            has_clut,
            width: 0,
            height: 0,
            clut_x: 0,
            clut_y: 0,
            clut_width: 0,
            clut_height: 0,
            pixel_data_size: 0,
            clut_data_size: 0,
        };

        if has_clut {
            if data.len() < 20 {
                return Err(DwExtractError::InvalidTim("Data too small for TIM with CLUT".to_string()));
            }
            
            let clut_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
            info.clut_data_size = clut_size;
            
            info.clut_x = u16::from_le_bytes([data[12], data[13]]);
            info.clut_y = u16::from_le_bytes([data[14], data[15]]);
            info.clut_width = u16::from_le_bytes([data[16], data[17]]);
            info.clut_height = u16::from_le_bytes([data[18], data[19]]);
            
            let pixel_data_offset = 20 + clut_size;
            if data.len() < pixel_data_offset + 8 {
                return Err(DwExtractError::InvalidTim("Data too small for pixel data header".to_string()));
            }
            
            info.width = u16::from_le_bytes([data[pixel_data_offset], data[pixel_data_offset + 1]]);
            info.height = u16::from_le_bytes([data[pixel_data_offset + 2], data[pixel_data_offset + 3]]);
            info.pixel_data_size = u32::from_le_bytes([
                data[pixel_data_offset + 4], data[pixel_data_offset + 5],
                data[pixel_data_offset + 6], data[pixel_data_offset + 7]
            ]) as usize;
        } else {
            if data.len() < 12 {
                return Err(DwExtractError::InvalidTim("Data too small for TIM without CLUT".to_string()));
            }
            
            info.width = u16::from_le_bytes([data[8], data[9]]);
            info.height = u16::from_le_bytes([data[10], data[11]]);
            info.pixel_data_size = (data.len() - 12).min((info.width as usize * info.height as usize * bpp as usize) / 8);
        }

        Ok(info)
    }

    pub fn validate(data: &[u8]) -> Result<TimInfo> {
        let info = Self::parse(data)?;
        
        if info.width == 0 || info.height == 0 {
            return Err(DwExtractError::InvalidTim("Invalid dimensions".to_string()));
        }
        
        if info.bpp != 4 && info.bpp != 8 && info.bpp != 16 && info.bpp != 24 {
            return Err(DwExtractError::InvalidTim(format!("Unsupported BPP: {}", info.bpp)));
        }

        Ok(info)
    }

    pub fn decode_image(data: &[u8]) -> Result<TimImage> {
        let info = Self::parse(data)?;
        
        if info.width == 0 || info.height == 0 {
            return Err(DwExtractError::InvalidTim("Invalid dimensions".to_string()));
        }

        // Sanity check: PS1 textures max at 2048x2048
        const MAX_DIM: u16 = 2048;
        if info.width > MAX_DIM || info.height > MAX_DIM {
            return Err(DwExtractError::InvalidTim(format!("Dimensions too large: {}x{}", info.width, info.height)));
        }

        let mut clut = None;
        let pixel_data_offset;

        if info.has_clut {
            let clut_size = info.clut_data_size;
            let clut_data = &data[20..20 + clut_size];
            clut = Some(clut_data.to_vec());
            pixel_data_offset = 20 + clut_size + 8;
        } else {
            pixel_data_offset = 12;
        }

        let pixel_data = &data[pixel_data_offset..];
        
        Ok(TimImage {
            info,
            clut,
            pixel_data: pixel_data.to_vec(),
        })
    }

    pub fn to_png(data: &[u8], output_path: &std::path::Path) -> Result<()> {
        let tim_image = Self::decode_image(data)?;
        let info = &tim_image.info;
        let width = info.width as u32;
        let height = info.height as u32;

        match info.bpp {
            4 => {
                let img = Self::decode_4bpp(&tim_image, width, height)?;
                img.save(output_path)
                    .map_err(|e| DwExtractError::InvalidTim(format!("Failed to save PNG: {}", e)))?;
            },
            8 => {
                let img = Self::decode_8bpp(&tim_image, width, height)?;
                img.save(output_path)
                    .map_err(|e| DwExtractError::InvalidTim(format!("Failed to save PNG: {}", e)))?;
            },
            16 => {
                let img = Self::decode_16bpp(&tim_image, width, height)?;
                img.save(output_path)
                    .map_err(|e| DwExtractError::InvalidTim(format!("Failed to save PNG: {}", e)))?;
            },
            24 => {
                let img = Self::decode_24bpp(&tim_image, width, height)?;
                img.save(output_path)
                    .map_err(|e| DwExtractError::InvalidTim(format!("Failed to save PNG: {}", e)))?;
            },
            _ => return Err(DwExtractError::InvalidTim(format!("Unsupported BPP for PNG conversion: {}", info.bpp))),
        }

        Ok(())
    }

    fn decode_4bpp(tim: &TimImage, width: u32, height: u32) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let clut = tim.clut.as_ref().ok_or_else(|| 
            DwExtractError::InvalidTim("4bpp TIM requires CLUT".to_string()))?;
        
        if clut.len() < 32 {
            return Err(DwExtractError::InvalidTim("CLUT too small for 4bpp".to_string()));
        }

        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;
        let bytes_per_row = (width + 1) / 2;

        for y in 0..height {
            let row_offset = (y as usize) * (bytes_per_row as usize);
            for x in 0..width {
                let byte_idx = row_offset + (x as usize / 2);
                if byte_idx >= pixel_data.len() { continue; }
                
                let byte = pixel_data[byte_idx];
                let palette_idx = if x % 2 == 0 { byte & 0x0F } else { byte >> 4 };
                
                let clut_offset = (palette_idx as usize) * 2;
                if clut_offset + 1 >= clut.len() { continue; }
                
                let color = u16::from_le_bytes([clut[clut_offset], clut[clut_offset + 1]]);
                let (r, g, b, a) = Self::rgb555_to_rgba(color);
                
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        Ok(img)
    }

    fn decode_8bpp(tim: &TimImage, width: u32, height: u32) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let clut = tim.clut.as_ref().ok_or_else(|| 
            DwExtractError::InvalidTim("8bpp TIM requires CLUT".to_string()))?;
        
        if clut.len() < 512 {
            return Err(DwExtractError::InvalidTim("CLUT too small for 8bpp".to_string()));
        }

        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;

        for y in 0..height {
            let row_offset = (y as usize) * (width as usize);
            for x in 0..width {
                let byte_idx = row_offset + (x as usize);
                if byte_idx >= pixel_data.len() { continue; }
                
                let palette_idx = pixel_data[byte_idx] as usize;
                let clut_offset = palette_idx * 2;
                if clut_offset + 1 >= clut.len() { continue; }
                
                let color = u16::from_le_bytes([clut[clut_offset], clut[clut_offset + 1]]);
                let (r, g, b, a) = Self::rgb555_to_rgba(color);
                
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        Ok(img)
    }

    fn decode_16bpp(tim: &TimImage, width: u32, height: u32) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;

        for y in 0..height {
            let row_offset = (y as usize) * (width as usize) * 2;
            for x in 0..width {
                let byte_idx = row_offset + (x as usize) * 2;
                if byte_idx + 1 >= pixel_data.len() { continue; }
                
                let color = u16::from_le_bytes([pixel_data[byte_idx], pixel_data[byte_idx + 1]]);
                let (r, g, b, a) = Self::rgb555_to_rgba(color);
                
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        Ok(img)
    }

    fn decode_24bpp(tim: &TimImage, width: u32, height: u32) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;

        for y in 0..height {
            let row_offset = (y as usize) * (width as usize) * 3;
            for x in 0..width {
                let byte_idx = row_offset + (x as usize) * 3;
                if byte_idx + 2 >= pixel_data.len() { continue; }
                
                let r = pixel_data[byte_idx];
                let g = pixel_data[byte_idx + 1];
                let b = pixel_data[byte_idx + 2];
                
                img.put_pixel(x, y, Rgb([r, g, b]));
            }
        }
        Ok(img)
    }

    fn rgb555_to_rgba(color: u16) -> (u8, u8, u8, u8) {
        let r = ((color >> 10) & 0x1F) as u8;
        let g = ((color >> 5) & 0x1F) as u8;
        let b = (color & 0x1F) as u8;
        let a = if (color & 0x8000) != 0 { 0 } else { 255 };
        
        let r = (r << 3) | (r >> 2);
        let g = (g << 3) | (g >> 2);
        let b = (b << 3) | (b >> 2);
        
        (r, g, b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tim_header_parsing() {
        let tim_with_clut = vec![
            0x10, 0x00, 0x00, 0x00,  // ID = 0x10
            0x09, 0x00, 0x00, 0x00,  // Flags: 4bpp + CLUT
            0x20, 0x00, 0x00, 0x00,  // CLUT size = 32
            0x00, 0x00,              // CLUT X
            0x00, 0x00,              // CLUT Y
            0x10, 0x00,              // CLUT width = 16
            0x01, 0x00,              // CLUT height = 1
            0x40, 0x01,              // Width = 320
            0x00, 0x01,              // Height = 256
            0x00, 0x80, 0x00, 0x00,  // Pixel data size = 32768
        ];
        
        let info = TimHeader::validate(&tim_with_clut).unwrap();
        assert_eq!(info.bpp, 4);
        assert!(info.has_clut);
        assert_eq!(info.width, 320);
        assert_eq!(info.height, 256);
    }
}