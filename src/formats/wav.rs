use crate::error::{DwExtractError, Result};
use std::fs::File;
use std::io::{Write, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WavHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl WavHeader {
    pub fn new(sample_rate: u32, channels: u16, bits_per_sample: u16) -> Self {
        Self {
            sample_rate,
            channels,
            bits_per_sample,
        }
    }

    pub fn byte_rate(&self) -> u32 {
        self.sample_rate * self.channels as u32 * self.bits_per_sample as u32 / 8
    }

    pub fn block_align(&self) -> u16 {
        self.channels * self.bits_per_sample / 8
    }

    pub fn data_size(&self, num_samples: usize) -> u32 {
        (num_samples * self.channels as usize * self.bits_per_sample as usize / 8) as u32
    }

    pub fn riff_size(&self, num_samples: usize) -> u32 {
        36 + self.data_size(num_samples)
    }
}

pub struct WavWriter<W: Write + Seek> {
    writer: W,
    header: WavHeader,
    samples_written: usize,
    data_start: u64,
}

impl<W: Write + Seek> WavWriter<W> {
    pub fn new(writer: W, header: WavHeader) -> Result<Self> {
        let mut w = Self {
            writer,
            header,
            samples_written: 0,
            data_start: 0,
        };
        w.write_header(0)?;
        w.data_start = w.writer.stream_position()?;
        Ok(w)
    }

    fn write_header(&mut self, data_size: u32) -> Result<()> {
        let riff_size = 36 + data_size;

        self.writer.write_all(b"RIFF")?;
        self.writer.write_all(&riff_size.to_le_bytes())?;
        self.writer.write_all(b"WAVE")?;

        self.writer.write_all(b"fmt ")?;
        self.writer.write_all(&16u32.to_le_bytes())?;
        self.writer.write_all(&1u16.to_le_bytes())?;
        self.writer.write_all(&self.header.channels.to_le_bytes())?;
        self.writer.write_all(&self.header.sample_rate.to_le_bytes())?;
        self.writer.write_all(&self.header.byte_rate().to_le_bytes())?;
        self.writer.write_all(&self.header.block_align().to_le_bytes())?;
        self.writer.write_all(&self.header.bits_per_sample.to_le_bytes())?;

        self.writer.write_all(b"data")?;
        self.writer.write_all(&data_size.to_le_bytes())?;

        Ok(())
    }

    pub fn write_samples(&mut self, samples: &[i16]) -> Result<()> {
        for &sample in samples {
            self.writer.write_all(&sample.to_le_bytes())?;
        }
        self.samples_written += samples.len();
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        // The samples of a stereo stream come interleaved, so the data holds two bytes per sample of the slice.
        let data_size = (self.samples_written * 2) as u32;
        let pos = self.writer.stream_position()?;
        self.writer.seek(SeekFrom::Start(0))?;
        self.write_header(data_size)?;
        self.writer.seek(SeekFrom::Start(pos))?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn samples_written(&self) -> usize {
        self.samples_written
    }
}

pub fn write_wav<P: AsRef<Path>>(
    path: P,
    samples: &[i16],
    sample_rate: u32,
    channels: u16,
) -> Result<()> {
    let header = WavHeader::new(sample_rate, channels, 16);
    let file = File::create(path)?;
    let mut writer = WavWriter::new(file, header)?;
    writer.write_samples(samples)?;
    writer.finish()?;
    Ok(())
}

pub fn write_wav_stereo<P: AsRef<Path>>(
    path: P,
    left: &[i16],
    right: &[i16],
    sample_rate: u32,
) -> Result<()> {
    if left.len() != right.len() {
        return Err(DwExtractError::InvalidFormat("Left and right channels must have same length".into()));
    }

    let header = WavHeader::new(sample_rate, 2, 16);
    let file = File::create(path)?;
    let mut writer = WavWriter::new(file, header)?;

    for i in 0..left.len() {
        writer.write_samples(&[left[i], right[i]])?;
    }

    writer.finish()?;
    Ok(())
}

pub fn vag_to_wav<P: AsRef<Path>>(
    vag_data: &[u8],
    path: P,
    sample_rate: u32,
) -> Result<()> {
    let samples = crate::formats::adpcm::decode_vag(vag_data)?;
    write_wav(path, &samples, sample_rate, 1)
}

/// Decodes a stream of XA sound groups (18 x 128 bytes each) and writes it as a WAV file.
pub fn xa_adpcm_to_wav<P: AsRef<Path>>(
    xa_data: &[u8],
    coding: &crate::formats::video::xa_adpcm::XaCoding,
    path: P,
) -> Result<()> {
    use crate::formats::video::xa_adpcm::{XaDecoder, XA_SOUND_GROUPS, XA_SOUND_GROUP_SIZE};

    let mut decoder = XaDecoder::new();
    let mut samples = Vec::new();
    for sector in xa_data.chunks(XA_SOUND_GROUPS * XA_SOUND_GROUP_SIZE) {
        decoder.decode_sector(sector, coding, &mut samples)?;
    }
    write_wav(path, &samples, coding.sample_rate, coding.channels as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_wav_header() {
        let header = WavHeader::new(44100, 2, 16);
        assert_eq!(header.byte_rate(), 176400);
        assert_eq!(header.block_align(), 4);
        assert_eq!(header.data_size(44100), 176400);
    }

    #[test]
    fn test_write_wav() {
        let mut buf = Cursor::new(Vec::new());
        let header = WavHeader::new(44100, 1, 16);
        let mut writer = WavWriter::new(&mut buf, header).unwrap();
        writer.write_samples(&[0, 100, -100, 200, -200]).unwrap();
        writer.finish().unwrap();

        let data = buf.into_inner();
        assert!(data.starts_with(b"RIFF"));
        assert!(&data[8..12] == b"WAVE");
    }
}