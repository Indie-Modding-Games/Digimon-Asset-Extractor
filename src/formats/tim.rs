use crate::error::{DwExtractError, Result};
use image::{ImageBuffer, Rgba};

/// The magic word of a TIM file.
const TIM_MAGIC: u32 = 0x10;

/// The size of the header of a block: length, x, y, width and height.
const BLOCK_HEADER_SIZE: usize = 12;

/// The limits of the VRAM, in 16-bit units.
const VRAM_WIDTH: u16 = 1024;
const VRAM_HEIGHT: u16 = 512;

/// The pixels of a palette: 16 in a 4-bit image, 256 in an 8-bit one.
fn palette_size(bpp: u8) -> usize {
    if bpp == 4 {
        16
    } else {
        256
    }
}

#[derive(Debug, Clone)]
pub struct TimInfo {
    /// The bits of each pixel: 4, 8, 16 or 24.
    pub bpp: u8,
    pub has_clut: bool,
    /// The size of the image in pixels.
    pub width: u16,
    pub height: u16,
    /// The size of the image in the units of the file, where a unit is 16 bits of VRAM.
    pub vram_width: u16,
    pub vram_height: u16,
    pub clut_x: u16,
    pub clut_y: u16,
    pub clut_width: u16,
    pub clut_height: u16,
    pub pixel_data_size: usize,
    pub clut_data_size: usize,
    /// The number of palettes of the CLUT: one per group of colours in a row.
    pub palette_count: usize,
}

/// A block of a TIM file: the length, the position and the size in the VRAM, and the data.
#[derive(Debug, Clone)]
struct TimBlock {
    length: usize,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    data: Vec<u8>,
}

/// The header of a TIM file: the magic word and the flags.
#[derive(Debug, Clone)]
pub struct TimHeader {
    pub id: u32,
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct TimImage {
    pub info: TimInfo,
    /// The colours of the CLUT, two bytes each.
    pub clut: Option<Vec<u8>>,
    pub pixel_data: Vec<u8>,
}

impl TimHeader {
    /// Reads the magic word and the flags of the file.
    pub fn read(data: &[u8]) -> Result<TimHeader> {
        if data.len() < 8 {
            return Err(DwExtractError::InvalidTim(
                "Data too small for TIM header".to_string(),
            ));
        }

        let id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if id != TIM_MAGIC {
            return Err(DwExtractError::InvalidTim(format!(
                "Invalid TIM ID: 0x{:08X}, expected 0x{:08X}",
                id, TIM_MAGIC
            )));
        }

        let flags = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        Ok(TimHeader { id, flags })
    }

    /// Reads a block: `{u32 length; u16 x, y, width, height; data}`. The length counts the
    /// header of the block, and the data is as long as the size of the block in the VRAM.
    fn read_block(data: &[u8], offset: usize, name: &str) -> Result<TimBlock> {
        if data.len() < offset + BLOCK_HEADER_SIZE {
            return Err(DwExtractError::InvalidTim(format!(
                "Data too small for the {} block header",
                name
            )));
        }

        let length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        let x = u16::from_le_bytes([data[offset + 4], data[offset + 5]]);
        let y = u16::from_le_bytes([data[offset + 6], data[offset + 7]]);
        let width = u16::from_le_bytes([data[offset + 8], data[offset + 9]]);
        let height = u16::from_le_bytes([data[offset + 10], data[offset + 11]]);

        if length == 0 || width == 0 || height == 0 {
            return Err(DwExtractError::InvalidTim(format!(
                "The {} block has an invalid size {}x{}",
                name, width, height
            )));
        }

        if width > VRAM_WIDTH || height > VRAM_HEIGHT {
            return Err(DwExtractError::InvalidTim(format!(
                "The {} block has an invalid size {}x{}",
                name, width, height
            )));
        }

        // The data of a block is as long as its size in the VRAM, where a unit is 16 bits.
        let data_size = width as usize * 2 * height as usize;

        if length < BLOCK_HEADER_SIZE + data_size {
            return Err(DwExtractError::InvalidTim(format!(
                "The {} block length {} is too small for {}x{}",
                name, length, width, height
            )));
        }

        if offset + length > data.len() {
            return Err(DwExtractError::InvalidTim(format!(
                "The {} block length {} runs past the end of the file",
                name, length
            )));
        }

        Ok(TimBlock {
            length,
            x,
            y,
            width,
            height,
            data: data[offset + BLOCK_HEADER_SIZE..offset + BLOCK_HEADER_SIZE + data_size].to_vec(),
        })
    }

    /// The size of an image in pixels, from its size in the units of the file.
    fn pixel_width(bpp: u8, vram_width: u16) -> u32 {
        match bpp {
            4 => vram_width as u32 * 4,
            8 => vram_width as u32 * 2,
            16 => vram_width as u32,
            24 => vram_width as u32 * 2 / 3,
            _ => 0,
        }
    }

    pub fn parse(data: &[u8]) -> Result<TimInfo> {
        if data.len() < 8 {
            return Err(DwExtractError::InvalidTim(
                "Data too small for TIM header".to_string(),
            ));
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != TIM_MAGIC {
            return Err(DwExtractError::InvalidTim(format!(
                "Invalid TIM ID: 0x{:08X}, expected 0x{:08X}",
                magic, TIM_MAGIC
            )));
        }

        let flags = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let bpp = match flags & 0x03 {
            0 => 4,
            1 => 8,
            2 => 16,
            3 => 24,
            _ => unreachable!(),
        };

        let has_clut = (flags & 0x08) != 0;

        let mut offset = 8;
        let mut clut_rect = (0u16, 0u16, 0u16, 0u16);
        let mut clut_data_size = 0;

        if has_clut {
            let clut = Self::read_block(data, offset, "CLUT")?;
            clut_rect = (clut.x, clut.y, clut.width, clut.height);
            clut_data_size = clut.data.len();
            offset += clut.length;
        }

        let image = Self::read_block(data, offset, "image")?;

        let width = Self::pixel_width(bpp, image.width);
        let height = image.height as u32;

        // The palettes of the CLUT: one per group of colours in a row.
        let palette_count = if has_clut {
            let size = palette_size(bpp);
            (clut_rect.2 as usize / size) * clut_rect.3 as usize
        } else {
            0
        };

        Ok(TimInfo {
            bpp,
            has_clut,
            width: width as u16,
            height: height as u16,
            vram_width: image.width,
            vram_height: image.height,
            clut_x: clut_rect.0,
            clut_y: clut_rect.1,
            clut_width: clut_rect.2,
            clut_height: clut_rect.3,
            pixel_data_size: image.data.len(),
            clut_data_size,
            palette_count,
        })
    }

    pub fn validate(data: &[u8]) -> Result<TimInfo> {
        let info = Self::parse(data)?;

        if info.width == 0 || info.height == 0 {
            return Err(DwExtractError::InvalidTim("Invalid dimensions".to_string()));
        }

        if info.bpp != 4 && info.bpp != 8 && info.bpp != 16 && info.bpp != 24 {
            return Err(DwExtractError::InvalidTim(format!(
                "Unsupported BPP: {}",
                info.bpp
            )));
        }

        Ok(info)
    }

    pub fn decode_image(data: &[u8]) -> Result<TimImage> {
        let info = Self::parse(data)?;

        let mut offset = 8;
        let mut clut = None;

        if info.has_clut {
            let block = Self::read_block(data, offset, "CLUT")?;
            clut = Some(block.data);
            offset += block.length;
        }

        let image = Self::read_block(data, offset, "image")?;

        Ok(TimImage {
            info,
            clut,
            pixel_data: image.data,
        })
    }

    /// The colours of one palette of the CLUT: the group of colours that starts at the given
    /// index, taken from the rows of the CLUT.
    fn palette(clut: &[u8], bpp: u8, clut_width: u16, index: usize) -> Option<Vec<u16>> {
        let size = palette_size(bpp);
        if clut_width as usize % size != 0 {
            return None;
        }

        let per_row = clut_width as usize / size;
        let start = (index / per_row) * clut_width as usize + (index % per_row) * size;

        let mut colors = Vec::with_capacity(size);
        for entry in 0..size {
            let at = (start + entry) * 2;
            if at + 1 >= clut.len() {
                return None;
            }
            colors.push(u16::from_le_bytes([clut[at], clut[at + 1]]));
        }

        Some(colors)
    }

    /// Writes the image as a PNG. With more than one palette, the file name gets the number of
    /// the palette: `image.png` or `image_p0.png`.
    pub fn to_png(data: &[u8], output_path: &std::path::Path) -> Result<()> {
        let files = Self::to_pngs(data, output_path)?;
        if files.is_empty() {
            return Err(DwExtractError::InvalidTim(
                "The TIM has no image to write".to_string(),
            ));
        }
        Ok(())
    }

    /// Writes the image as a PNG, one file per palette, and returns the files written.
    pub fn to_pngs(data: &[u8], output_path: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
        let tim = Self::decode_image(data)?;
        let info = &tim.info;

        let mut written = Vec::new();

        if info.bpp == 4 || info.bpp == 8 {
            let clut = tim.clut.as_ref().ok_or_else(|| {
                DwExtractError::InvalidTim(format!("{}bpp TIM requires a CLUT", info.bpp))
            })?;

            let palettes = info.palette_count.max(1);
            let multiple = palettes > 1;

            for index in 0..palettes {
                let colors = Self::palette(clut, info.bpp, info.clut_width, index).ok_or_else(|| {
                    DwExtractError::InvalidTim(format!("The CLUT has no palette {}", index))
                })?;

                let img = match info.bpp {
                    4 => Self::decode_4bpp(&tim, &colors)?,
                    _ => Self::decode_8bpp(&tim, &colors)?,
                };

                let path = if multiple {
                    palette_path(output_path, index)
                } else {
                    output_path.to_path_buf()
                };

                img.save(&path).map_err(|e| {
                    DwExtractError::InvalidTim(format!("Failed to save PNG: {}", e))
                })?;

                written.push(path);
            }
        } else {
            let img = match info.bpp {
                16 => Self::decode_16bpp(&tim)?,
                _ => Self::decode_24bpp(&tim)?,
            };

            img.save(output_path)
                .map_err(|e| DwExtractError::InvalidTim(format!("Failed to save PNG: {}", e)))?;

            written.push(output_path.to_path_buf());
        }

        Ok(written)
    }

    fn decode_4bpp(tim: &TimImage, palette: &[u16]) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let width = tim.info.width as u32;
        let height = tim.info.height as u32;

        if palette.len() < 16 {
            return Err(DwExtractError::InvalidTim(
                "CLUT too small for 4bpp".to_string(),
            ));
        }

        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;

        // Each byte holds two pixels: the first one in the low nibble.
        let bytes_per_row = tim.info.vram_width as usize * 2;

        for y in 0..height {
            let row_offset = (y as usize) * bytes_per_row;
            for x in 0..width {
                let byte_idx = row_offset + (x as usize / 2);
                if byte_idx >= pixel_data.len() {
                    break;
                }

                let byte = pixel_data[byte_idx];
                let palette_idx = if x % 2 == 0 { byte & 0x0F } else { byte >> 4 } as usize;

                if palette_idx >= palette.len() {
                    return Err(DwExtractError::InvalidTim(format!(
                        "TIM colour index {} is outside its {}-colour palette",
                        palette_idx,
                        palette.len()
                    )));
                }

                let (r, g, b, a) = Self::rgb555_to_rgba(palette[palette_idx]);
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        Ok(img)
    }

    fn decode_8bpp(tim: &TimImage, palette: &[u16]) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let width = tim.info.width as u32;
        let height = tim.info.height as u32;

        if palette.len() < 256 {
            return Err(DwExtractError::InvalidTim(
                "CLUT too small for 8bpp".to_string(),
            ));
        }

        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;
        let bytes_per_row = tim.info.vram_width as usize * 2;

        for y in 0..height {
            let row_offset = (y as usize) * bytes_per_row;
            for x in 0..width {
                let byte_idx = row_offset + x as usize;
                if byte_idx >= pixel_data.len() {
                    break;
                }

                let palette_idx = pixel_data[byte_idx] as usize;
                if palette_idx >= palette.len() {
                    return Err(DwExtractError::InvalidTim(format!(
                        "TIM colour index {} is outside its {}-colour palette",
                        palette_idx,
                        palette.len()
                    )));
                }

                let (r, g, b, a) = Self::rgb555_to_rgba(palette[palette_idx]);
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        Ok(img)
    }

    fn decode_16bpp(tim: &TimImage) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let width = tim.info.width as u32;
        let height = tim.info.height as u32;

        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;
        let bytes_per_row = tim.info.vram_width as usize * 2;

        for y in 0..height {
            let row_offset = (y as usize) * bytes_per_row;
            for x in 0..width {
                let byte_idx = row_offset + (x as usize) * 2;
                if byte_idx + 1 >= pixel_data.len() {
                    break;
                }

                let color = u16::from_le_bytes([pixel_data[byte_idx], pixel_data[byte_idx + 1]]);
                let (r, g, b, a) = Self::rgb555_to_rgba(color);
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        Ok(img)
    }

    fn decode_24bpp(tim: &TimImage) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let width = tim.info.width as u32;
        let height = tim.info.height as u32;

        let mut img = ImageBuffer::new(width, height);
        let pixel_data = &tim.pixel_data;
        let bytes_per_row = tim.info.vram_width as usize * 2;

        for y in 0..height {
            let row_offset = (y as usize) * bytes_per_row;
            for x in 0..width {
                let byte_idx = row_offset + (x as usize) * 3;
                if byte_idx + 2 >= pixel_data.len() {
                    break;
                }

                img.put_pixel(
                    x,
                    y,
                    Rgba([
                        pixel_data[byte_idx],
                        pixel_data[byte_idx + 1],
                        pixel_data[byte_idx + 2],
                        255,
                    ]),
                );
            }
        }
        Ok(img)
    }

    fn rgb555_to_rgba(color: u16) -> (u8, u8, u8, u8) {
        let r = ((color >> 10) & 0x1F) as u8;
        let g = ((color >> 5) & 0x1F) as u8;
        let b = (color & 0x1F) as u8;
        let a = if color == 0 { 0 } else { 255 };

        let r = (r << 3) | (r >> 2);
        let g = (g << 3) | (g >> 2);
        let b = (b << 3) | (b >> 2);

        (r, g, b, a)
    }
}

/// The path of a file with the number of the palette: `image.png` becomes `image_p0.png`.
pub fn palette_path(path: &std::path::Path, index: usize) -> std::path::PathBuf {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".to_string());

    path.with_file_name(format!("{}_p{}.png", stem, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A TIM file with a CLUT and an image, built the way the reference reads them.
    fn tim(bpp: u8, clut_width: u16, clut_height: u16, vram_width: u16, vram_height: u16) -> Vec<u8> {
        let mut data = Vec::new();

        let flags = match bpp {
            4 => 0x08u32,
            8 => 0x09,
            16 => 0x02,
            _ => 0x03,
        };
        data.extend_from_slice(&TIM_MAGIC.to_le_bytes());
        data.extend_from_slice(&flags.to_le_bytes());

        if bpp == 4 || bpp == 8 {
            let clut_data_size = clut_width as usize * 2 * clut_height as usize;
            let clut_length = BLOCK_HEADER_SIZE + clut_data_size;
            data.extend_from_slice(&(clut_length as u32).to_le_bytes());
            data.extend_from_slice(&256u16.to_le_bytes()); // x
            data.extend_from_slice(&480u16.to_le_bytes()); // y
            data.extend_from_slice(&clut_width.to_le_bytes());
            data.extend_from_slice(&clut_height.to_le_bytes());
            for index in 0..clut_data_size / 2 {
                data.extend_from_slice(&(index as u16).to_le_bytes());
            }
        }

        let image_data_size = vram_width as usize * 2 * vram_height as usize;
        let image_length = BLOCK_HEADER_SIZE + image_data_size;
        data.extend_from_slice(&(image_length as u32).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes()); // x
        data.extend_from_slice(&0u16.to_le_bytes()); // y
        data.extend_from_slice(&vram_width.to_le_bytes());
        data.extend_from_slice(&vram_height.to_le_bytes());
        for index in 0..image_data_size {
            data.push((index % 16) as u8);
        }

        data
    }

    #[test]
    fn test_tim_header_parsing() {
        // A 4-bit image of 16x16 pixels: 4 units of VRAM wide and 16 high.
        let data = tim(4, 16, 1, 4, 16);

        let info = TimHeader::validate(&data).unwrap();
        assert_eq!(info.bpp, 4);
        assert!(info.has_clut);
        assert_eq!(info.width, 16);
        assert_eq!(info.height, 16);
        assert_eq!(info.vram_width, 4);
        assert_eq!(info.clut_width, 16);
        assert_eq!(info.palette_count, 1);
        assert_eq!(info.pixel_data_size, 4 * 2 * 16);
    }

    #[test]
    fn test_tim_with_several_palettes() {
        // A CLUT of 32 colours holds two palettes of 16 colours.
        let data = tim(4, 32, 1, 4, 16);

        let info = TimHeader::validate(&data).unwrap();
        assert_eq!(info.clut_width, 32);
        assert_eq!(info.palette_count, 2);
    }

    #[test]
    fn test_tim_8bpp_is_twice_the_units() {
        let data = tim(8, 256, 1, 16, 8);

        let info = TimHeader::validate(&data).unwrap();
        assert_eq!(info.bpp, 8);
        assert_eq!(info.width, 32);
        assert_eq!(info.height, 8);
        assert_eq!(info.palette_count, 1);
    }

    #[test]
    fn test_tim_without_clut() {
        let data = tim(16, 0, 0, 16, 16);

        let info = TimHeader::validate(&data).unwrap();
        assert!(!info.has_clut);
        assert_eq!(info.bpp, 16);
        assert_eq!(info.width, 16);
        assert_eq!(info.height, 16);
    }

    #[test]
    fn test_tim_with_a_wrong_magic() {
        let mut data = tim(4, 16, 1, 4, 16);
        data[0] = 0x11;
        assert!(TimHeader::validate(&data).is_err());
    }

    #[test]
    fn test_tim_with_a_truncated_block() {
        let mut data = tim(4, 16, 1, 4, 16);
        data.truncate(data.len() - 16);
        assert!(TimHeader::validate(&data).is_err());
    }

    #[test]
    fn test_tim_png_has_the_size_of_the_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.png");

        let data = tim(4, 16, 1, 4, 16);
        TimHeader::to_png(&data, &path).unwrap();

        let image = image::open(&path).unwrap();
        assert_eq!(image.width(), 16);
        assert_eq!(image.height(), 16);
    }

    #[test]
    fn test_tim_with_several_palettes_writes_a_file_each() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.png");

        let data = tim(4, 32, 1, 4, 16);
        let files = TimHeader::to_pngs(&data, &path).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("image_p0.png"));
        assert!(files[1].ends_with("image_p1.png"));
        assert!(!path.exists());
    }
}
