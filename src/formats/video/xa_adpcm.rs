use crate::error::{DwExtractError, Result};

/// CD-ROM XA ADPCM audio, the sound of interleaved streams: each form 2 audio sector holds 18 sound groups of 128
/// bytes (16 bytes of parameters, then 28 words of packed samples), which the drive decodes itself.

/// Sound groups in a sector and the bytes each one takes; the 20 bytes after them are unused.
pub const XA_SOUND_GROUPS: usize = 18;
pub const XA_SOUND_GROUP_SIZE: usize = 128;
/// Samples each sound unit gives.
pub const XA_SAMPLES_PER_UNIT: usize = 28;

const GROUP_HEADER_SIZE: usize = 16;
const MAX_UNITS: usize = 8;

const CODING_STEREO: u8 = 0x01;
const CODING_HALF_RATE: u8 = 0x04;
const CODING_EIGHT_BIT: u8 = 0x10;
const CODING_EMPHASIS: u8 = 0x40;
const CODING_RESERVED: u8 = 0xAA;

/// The prediction filters, in 64ths: the weight of the previous sample and of the one before it.
const FILTERS: [[i32; 2]; 4] = [[0, 0], [60, 0], [115, -52], [98, -55]];

/// How an audio sector is coded: the coding byte of its subheader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XaCoding {
    /// 1 or 2 (bit 0).
    pub channels: u8,
    /// 37800 or 18900 Hz (bit 2).
    pub sample_rate: u32,
    /// 4 or 8 (bit 4).
    pub bits_per_sample: u8,
}

impl XaCoding {
    /// The sample frames (one sample per channel) a sector gives: 4032 samples over the channels at 4 bits, 2016 at 8.
    pub fn sample_frames_per_sector(&self) -> usize {
        let units = if self.bits_per_sample == 4 { 8 } else { 4 };
        XA_SOUND_GROUPS * units * XA_SAMPLES_PER_UNIT / self.channels as usize
    }
}

/// Reads a coding byte. Reserved values (bits 1, 3, 5 and 7) and emphasis (bit 6), which the decoder does not apply,
/// are errors.
pub fn read_xa_coding(coding: u8) -> Result<XaCoding> {
    if coding & CODING_RESERVED != 0 {
        return Err(DwExtractError::InvalidFormat(format!(
            "the XA coding {:#04x} uses reserved bits",
            coding
        )));
    }
    if coding & CODING_EMPHASIS != 0 {
        return Err(DwExtractError::InvalidFormat(format!(
            "the XA coding {:#04x} asks for emphasis, which is not supported",
            coding
        )));
    }
    Ok(XaCoding {
        channels: if coding & CODING_STEREO != 0 { 2 } else { 1 },
        sample_rate: if coding & CODING_HALF_RATE != 0 { 18900 } else { 37800 },
        bits_per_sample: if coding & CODING_EIGHT_BIT != 0 { 8 } else { 4 },
    })
}

/// Shifts 13 to 15 are reserved and act as 9.
fn shift_of(parameter: u8) -> u32 {
    let shift = parameter & 0x0F;
    if shift > 12 {
        9
    } else {
        shift as u32
    }
}

/// Decodes the sectors of one stream in order. Each channel keeps the last two samples of its previous sound unit,
/// across sound groups and sectors, so the sectors of a stream must go through the same decoder.
#[derive(Debug, Clone, Default)]
pub struct XaDecoder {
    /// Per channel: the previous sample and the one before it.
    history: [[i32; 2]; 2],
}

impl XaDecoder {
    pub fn new() -> Self {
        Self {
            history: [[0; 2]; 2],
        }
    }

    /// Forgets the samples of the previous unit, as a new stream does.
    pub fn reset(&mut self) {
        self.history = [[0; 2]; 2];
    }

    /// Decodes the sound groups of one sector's user data (at least 18 x 128 bytes) and appends 16-bit samples to
    /// `out`, interleaved left and right when stereo. Returns how many sound units have a parameter byte that differs
    /// from its copy in the group header; the decoder uses the first one.
    pub fn decode_sector(
        &mut self,
        data: &[u8],
        coding: &XaCoding,
        out: &mut Vec<i16>,
    ) -> Result<usize> {
        if data.len() < XA_SOUND_GROUPS * XA_SOUND_GROUP_SIZE {
            return Err(DwExtractError::InvalidFormat(format!(
                "an XA sector of {} bytes is too short for 18 sound groups",
                data.len()
            )));
        }
        if (coding.channels != 1 && coding.channels != 2)
            || (coding.bits_per_sample != 4 && coding.bits_per_sample != 8)
        {
            return Err(DwExtractError::InvalidFormat(
                "unsupported XA coding".to_string(),
            ));
        }
        let four_bit = coding.bits_per_sample == 4;
        let stereo = coding.channels == 2;
        let units = if four_bit { 8 } else { 4 };
        let mut mismatches = 0;
        let mut samples = [[0i16; XA_SAMPLES_PER_UNIT]; MAX_UNITS];

        for group in 0..XA_SOUND_GROUPS {
            let bytes = &data[group * XA_SOUND_GROUP_SIZE..(group + 1) * XA_SOUND_GROUP_SIZE];
            for unit in 0..units {
                // The parameters at bytes 4..11; bytes 0..3 copy 4..7 and bytes 12..15 copy 8..11.
                let parameter = bytes[4 + unit];
                if parameter != bytes[if unit < 4 { unit } else { unit + 8 }] {
                    mismatches += 1;
                }
                let shift = shift_of(parameter);
                let filter = FILTERS[((parameter >> 4) & 0x03) as usize];
                let channel = if stereo { unit % 2 } else { 0 };
                for sample in 0..XA_SAMPLES_PER_UNIT {
                    let expanded: i32 = if four_bit {
                        let packed = bytes[GROUP_HEADER_SIZE + (sample * 4) + (unit / 2)];
                        let nibble = if unit % 2 == 0 {
                            packed & 0x0F
                        } else {
                            packed >> 4
                        };
                        ((nibble ^ 0x08) as i32 - 8) * 4096
                    } else {
                        (bytes[GROUP_HEADER_SIZE + (sample * 4) + unit] as i8) as i32 * 256
                    };
                    let predicted = (self.history[channel][0] * filter[0]
                        + self.history[channel][1] * filter[1]
                        + 32)
                        >> 6;
                    let value = ((expanded >> shift) + predicted).clamp(-32768, 32767);
                    self.history[channel][1] = self.history[channel][0];
                    self.history[channel][0] = value;
                    samples[unit][sample] = value as i16;
                }
            }
            if stereo {
                for pair in 0..units / 2 {
                    for sample in 0..XA_SAMPLES_PER_UNIT {
                        out.push(samples[pair * 2][sample]);
                        out.push(samples[pair * 2 + 1][sample]);
                    }
                }
            } else {
                for unit in 0..units {
                    out.extend_from_slice(&samples[unit]);
                }
            }
        }
        Ok(mismatches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONO_FOUR_BIT: XaCoding = XaCoding {
        channels: 1,
        sample_rate: 37800,
        bits_per_sample: 4,
    };
    const STEREO_FOUR_BIT: XaCoding = XaCoding {
        channels: 2,
        sample_rate: 37800,
        bits_per_sample: 4,
    };
    const MONO_EIGHT_BIT: XaCoding = XaCoding {
        channels: 1,
        sample_rate: 37800,
        bits_per_sample: 8,
    };

    /// The user data of an audio sector: 18 sound groups, every unit silent with shift 12 and filter 0.
    fn silent_sector() -> Vec<u8> {
        let mut data = vec![0u8; 2324];
        for group in 0..XA_SOUND_GROUPS {
            for i in 0..16 {
                data[(group * XA_SOUND_GROUP_SIZE) + i] = 0x0C;
            }
        }
        data
    }

    /// Sets a 4-bit unit's parameter (and its copy) and all 28 of its nibbles in one sound group.
    fn set_unit(data: &mut [u8], group: usize, unit: usize, parameter: u8, nibble: u8) {
        let base = group * XA_SOUND_GROUP_SIZE;
        data[base + 4 + unit] = parameter;
        data[base + if unit < 4 { unit } else { unit + 8 }] = parameter;
        for sample in 0..XA_SAMPLES_PER_UNIT {
            let offset = base + 16 + (sample * 4) + (unit / 2);
            let old = data[offset];
            let value = if unit % 2 == 0 {
                (old & 0xF0) | nibble
            } else {
                (old & 0x0F) | (nibble << 4)
            };
            data[offset] = value;
        }
    }

    /// Decodes a mono 4-bit sector whose unit 0 is all 4096 and unit 1 is silent with this filter; returns unit 1's
    /// first two samples.
    fn after_step(filter: u8) -> (i16, i16) {
        let mut data = silent_sector();
        set_unit(&mut data, 0, 0, 0x00, 1);
        set_unit(&mut data, 0, 1, filter << 4, 0);
        let mut decoder = XaDecoder::new();
        let mut samples = Vec::new();
        decoder.decode_sector(&data, &MONO_FOUR_BIT, &mut samples).unwrap();
        (samples[28], samples[29])
    }

    #[test]
    fn the_coding_byte_gives_channels_rate_and_bits() {
        assert_eq!(read_xa_coding(0x01).unwrap(), STEREO_FOUR_BIT);
        assert_eq!(read_xa_coding(0x00).unwrap(), MONO_FOUR_BIT);
        assert_eq!(read_xa_coding(0x04).unwrap().sample_rate, 18900);
        assert_eq!(read_xa_coding(0x10).unwrap().bits_per_sample, 8);
        assert!(read_xa_coding(0x40).is_err());
        assert!(read_xa_coding(0x02).is_err());
        assert!(read_xa_coding(0x80).is_err());
        assert_eq!(STEREO_FOUR_BIT.sample_frames_per_sector(), 2016);
        assert_eq!(MONO_FOUR_BIT.sample_frames_per_sector(), 4032);
        assert_eq!(MONO_EIGHT_BIT.sample_frames_per_sector(), 2016);
    }

    #[test]
    fn a_sample_is_its_nibble_shifted_to_the_top_of_16_bits_and_right_by_the_shift() {
        let mut data = silent_sector();
        set_unit(&mut data, 0, 0, 0x00, 7);
        set_unit(&mut data, 0, 1, 0x00, 8);
        set_unit(&mut data, 0, 2, 0x0C, 1);
        set_unit(&mut data, 0, 3, 0x0D, 1); // shift 13 acts as 9
        let mut decoder = XaDecoder::new();
        let mut samples = Vec::new();
        let decoded = decoder.decode_sector(&data, &MONO_FOUR_BIT, &mut samples);
        assert!(decoded.is_ok(), "{:?}", decoded.err());
        assert_eq!(decoded.unwrap(), 0);
        assert_eq!(samples.len(), 4032);
        assert_eq!(samples[0], 28672);
        assert_eq!(samples[27], 28672);
        assert_eq!(samples[28], -32768);
        assert_eq!(samples[56], 1);
        assert_eq!(samples[84], 8);
        assert_eq!(samples[112], 0);
    }

    #[test]
    fn each_filter_predicts_from_the_previous_two_samples() {
        assert_eq!(after_step(0), (0, 0));
        assert_eq!(after_step(1), (3840, 3600));
        assert_eq!(after_step(2).0, 4032);
        assert_eq!(after_step(3).0, 2752);
    }

    #[test]
    fn predicted_samples_saturate_to_16_bits() {
        let mut data = silent_sector();
        set_unit(&mut data, 0, 0, 0x00, 7);
        set_unit(&mut data, 0, 1, 0x20, 7);
        let mut decoder = XaDecoder::new();
        let mut samples = Vec::new();
        decoder.decode_sector(&data, &MONO_FOUR_BIT, &mut samples).unwrap();
        assert_eq!(samples[28], 32767);
    }

    #[test]
    fn stereo_units_alternate_left_and_right_each_with_its_own_history() {
        let mut data = silent_sector();
        set_unit(&mut data, 0, 0, 0x00, 1);
        set_unit(&mut data, 0, 1, 0x00, 0xF);
        set_unit(&mut data, 0, 2, 0x10, 0);
        set_unit(&mut data, 0, 3, 0x10, 0);
        let mut decoder = XaDecoder::new();
        let mut samples = Vec::new();
        decoder.decode_sector(&data, &STEREO_FOUR_BIT, &mut samples).unwrap();
        assert_eq!(samples.len(), 4032);
        assert_eq!(samples[0], 4096);
        assert_eq!(samples[1], -4096);
        assert_eq!(samples[56], 3840);
        assert_eq!(samples[57], -3840);
    }

    #[test]
    fn the_history_carries_over_to_the_next_sector() {
        let mut first = silent_sector();
        set_unit(&mut first, XA_SOUND_GROUPS - 1, 7, 0x00, 1);
        let mut second = silent_sector();
        set_unit(&mut second, 0, 0, 0x10, 0);
        let mut decoder = XaDecoder::new();
        let mut samples = Vec::new();
        decoder.decode_sector(&first, &MONO_FOUR_BIT, &mut samples).unwrap();
        decoder.decode_sector(&second, &MONO_FOUR_BIT, &mut samples).unwrap();
        assert_eq!(samples[4031], 4096);
        assert_eq!(samples[4032], 3840);
    }

    #[test]
    fn eight_bit_units_hold_one_byte_per_sample() {
        let mut data = silent_sector();
        for sample in 0..XA_SAMPLES_PER_UNIT {
            data[16 + (sample * 4)] = 0x7F;
            data[16 + (sample * 4) + 1] = 0x80;
        }
        for unit in 0..4 {
            let parameter = if unit == 1 { 0x08 } else { 0x00 };
            data[unit] = parameter;
            data[4 + unit] = parameter;
        }
        let mut decoder = XaDecoder::new();
        let mut samples = Vec::new();
        decoder.decode_sector(&data, &MONO_EIGHT_BIT, &mut samples).unwrap();
        assert_eq!(samples.len(), 2016);
        assert_eq!(samples[0], 32512);
        assert_eq!(samples[28], -128);
    }

    #[test]
    fn parameters_that_differ_from_their_copies_are_counted_and_short_sectors_are_errors() {
        let mut data = silent_sector();
        data[4] = 0x00;
        data[XA_SOUND_GROUP_SIZE + 12] = 0x01;
        let mut decoder = XaDecoder::new();
        let mut samples = Vec::new();
        let mismatches = decoder.decode_sector(&data, &MONO_FOUR_BIT, &mut samples);
        assert!(mismatches.is_ok());
        assert_eq!(mismatches.unwrap(), 2);
        assert!(decoder
            .decode_sector(&data[..2303], &MONO_FOUR_BIT, &mut samples)
            .is_err());
        let unsupported = XaCoding {
            channels: 3,
            sample_rate: 37800,
            bits_per_sample: 4,
        };
        assert!(decoder.decode_sector(&data, &unsupported, &mut samples).is_err());
    }
}
