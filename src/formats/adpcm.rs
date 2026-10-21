use crate::error::{DwExtractError, Result};

const ADPCM_FILTERS: [[i16; 2]; 5] = [
    [0, 0],
    [60, 0],
    [115, -52],
    [98, -55],
    [122, -60],
];

/// The size of a block of samples.
pub const ADPCM_BLOCK_SIZE: usize = 16;
/// The number of samples a block holds.
pub const ADPCM_SAMPLES_PER_BLOCK: usize = 28;

const MAX_SHIFT: u8 = 12;
const RESERVED_SHIFT_VALUE: u8 = 9;
const MAX_FILTER: u8 = 4;
const RESERVED_HEADER_BIT: u8 = 0x80;

/// The flags of a block.
pub struct AdpcmFlags;

impl AdpcmFlags {
    /// The voice leaves the block for the loop start.
    pub const END: u8 = 0x01;
    /// With `END`: keep playing from the loop start; without: release.
    pub const REPEAT: u8 = 0x02;
    /// The voice takes this block as its loop start.
    pub const LOOP_START: u8 = 0x04;
}

/// A loop in samples: playback goes back to `start` after the sample before `end`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdpcmLoop {
    pub start: u32,
    pub end: u32,
}

/// A decoded sample.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdpcmSample {
    pub pcm: Vec<i16>,
    /// Blocks decoded, the end block included.
    pub blocks: usize,
    /// A block with the end flag was found; otherwise the data ran out first.
    pub terminated: bool,
    /// When the end block also has the repeat flag.
    pub loop_info: Option<AdpcmLoop>,
    /// The end block repeats but no block marks a loop start: the loop starts at 0.
    pub loop_start_implied: bool,
    /// Blocks with a reserved shift (13-15), filter (5-7) or header bit 7.
    pub reserved_blocks: usize,
    /// Bytes after the last whole block, when the data ran out.
    pub trailing_bytes: usize,
}

/// Decodes blocks up to the first one with the end flag, which is included, as the sound chip plays them: the residual
/// is the nibble as a sign-extended 16-bit value shifted by the block's shift, plus the prediction of the filter from
/// the last two samples, `(s1 * k0[f] + s2 * k1[f] + 32) >> 6`, clamped to 16 bits. Reserved values decode as the
/// hardware does: a shift above 12 as 9 and a filter above 4 as 4. The loop starts at the last block with the loop
/// start flag before the end, and ends after the end block.
pub fn decode_adpcm(data: &[u8]) -> AdpcmSample {
    let mut sample = AdpcmSample::default();
    let block_count = data.len() / ADPCM_BLOCK_SIZE;
    sample.pcm.reserve(block_count * ADPCM_SAMPLES_PER_BLOCK);

    let mut loop_start_block: Option<usize> = None;
    let mut previous1: i32 = 0;
    let mut previous2: i32 = 0;

    for index in 0..block_count {
        let block = &data[index * ADPCM_BLOCK_SIZE..(index + 1) * ADPCM_BLOCK_SIZE];
        let header = block[0];
        let flags = block[1];
        let shift_field = header & 0x0F;
        let filter_field = (header >> 4) & 0x07;
        if shift_field > MAX_SHIFT || filter_field > MAX_FILTER || (header & RESERVED_HEADER_BIT) != 0 {
            sample.reserved_blocks += 1;
        }
        let shift = if shift_field > MAX_SHIFT {
            RESERVED_SHIFT_VALUE
        } else {
            shift_field
        };
        let weights = ADPCM_FILTERS[std::cmp::min(filter_field, MAX_FILTER) as usize];

        if (flags & AdpcmFlags::LOOP_START) != 0 {
            loop_start_block = Some(index);
        }

        for position in 0..ADPCM_SAMPLES_PER_BLOCK {
            let packed = block[2 + position / 2] as u32;
            let nibble = ((if position % 2 == 0 { packed } else { packed >> 4 }) & 0x0F) as i16;
            // The nibble is the top of a 16-bit value: the cast sign-extends it.
            let residual = (((nibble as i32) << 12) as i16) >> shift;
            let predicted =
                residual as i32 + (((previous1 * weights[0] as i32) + (previous2 * weights[1] as i32) + 32) >> 6);
            let clamped = predicted.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            sample.pcm.push(clamped);
            previous2 = previous1;
            previous1 = clamped as i32;
        }

        sample.blocks += 1;

        if (flags & AdpcmFlags::END) != 0 {
            sample.terminated = true;
            if (flags & AdpcmFlags::REPEAT) != 0 {
                sample.loop_start_implied = loop_start_block.is_none();
                sample.loop_info = Some(AdpcmLoop {
                    start: loop_start_block.unwrap_or(0) as u32 * ADPCM_SAMPLES_PER_BLOCK as u32,
                    end: (index + 1) as u32 * ADPCM_SAMPLES_PER_BLOCK as u32,
                });
            }
            return sample;
        }
    }

    sample.trailing_bytes = data.len() % ADPCM_BLOCK_SIZE;
    sample
}

#[derive(Debug, Clone, Default, Copy, PartialEq)]
pub struct AdpcmState {
    pub prev1: i16,
    pub prev2: i16,
}

impl AdpcmState {
    pub fn new() -> Self {
        Self { prev1: 0, prev2: 0 }
    }

    pub fn reset(&mut self) {
        self.prev1 = 0;
        self.prev2 = 0;
    }
}

pub struct AdpcmDecoder {
    state: AdpcmState,
}

impl AdpcmDecoder {
    pub fn new() -> Self {
        Self {
            state: AdpcmState::new(),
        }
    }

    pub fn with_state(state: AdpcmState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> AdpcmState {
        self.state
    }

    pub fn reset(&mut self) {
        self.state.reset();
    }

    pub fn decode_block(&mut self, block: &[u8]) -> Result<Vec<i16>> {
        if block.len() != 16 {
            return Err(DwExtractError::InvalidFormat("ADPCM block must be 16 bytes".into()));
        }

        let header = block[0];
        let filter_idx = ((header >> 4) & 0x0F) as usize;
        let shift = header & 0x0F;

        if filter_idx >= ADPCM_FILTERS.len() {
            return Err(DwExtractError::InvalidFormat(format!("Invalid ADPCM filter index: {}", filter_idx)));
        }
        if shift > 12 {
            return Err(DwExtractError::InvalidFormat(format!("Invalid ADPCM shift: {}", shift)));
        }

        let [f0, f1] = ADPCM_FILTERS[filter_idx];

        let mut samples = Vec::with_capacity(28);
        for i in 1..15 {
            let byte = block[i];
            let nibble1 = (byte & 0x0F) as i16;
            let nibble2 = (byte >> 4) as i16;

            for &nibble in &[nibble1, nibble2] {
                let sample = self.decode_nibble(nibble, shift, f0, f1);
                samples.push(sample);
            }
        }

        Ok(samples)
    }

    fn decode_nibble(&mut self, nibble: i16, shift: u8, f0: i16, f1: i16) -> i16 {
        let mut sample = ((nibble << shift) as i32) << 11;
        sample = sample.saturating_sub((self.state.prev1 as i32 * f0 as i32) >> 6);
        sample = sample.saturating_sub((self.state.prev2 as i32 * f1 as i32) >> 6);
        sample = sample.clamp(-32768, 32767);

        self.state.prev2 = self.state.prev1;
        self.state.prev1 = sample as i16;

        sample as i16
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<i16>> {
        if data.len() % 16 != 0 {
            return Err(DwExtractError::InvalidFormat("ADPCM data length must be multiple of 16".into()));
        }

        let mut all_samples = Vec::new();
        for chunk in data.chunks(16) {
            all_samples.extend(self.decode_block(chunk)?);
        }
        Ok(all_samples)
    }
}

pub fn decode_vag(data: &[u8]) -> Result<Vec<i16>> {
    if data.len() % 16 != 0 {
        return Err(DwExtractError::InvalidFormat("ADPCM data length must be multiple of 16".into()));
    }
    Ok(decode_adpcm(data).pcm)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One ADPCM block: the header (filter << 4 | shift), the flags, then 28 nibbles.
    fn adpcm_block(filter: u8, shift: u8, flags: u8, nibbles: &[u8]) -> Vec<u8> {
        let mut block = vec![0u8; ADPCM_BLOCK_SIZE];
        block[0] = ((filter & 0x0F) << 4) | (shift & 0x0F);
        block[1] = flags;
        for position in 0..ADPCM_SAMPLES_PER_BLOCK {
            block[2 + position / 2] |= (nibbles[position] & 0x0F) << (4 * (position % 2));
        }
        block
    }

    fn nibbles_of(value: u8) -> Vec<u8> {
        vec![value; ADPCM_SAMPLES_PER_BLOCK]
    }

    /// A sample of blocks of constant nibbles, the flags given per block.
    fn adpcm_sample(flags: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for flag in flags {
            bytes.extend(adpcm_block(0, 12, *flag, &nibbles_of(1)));
        }
        bytes
    }

    #[test]
    fn test_adpcm_filter_table() {
        assert_eq!(ADPCM_FILTERS[0], [0, 0]);
        assert_eq!(ADPCM_FILTERS[1], [60, 0]);
        assert_eq!(ADPCM_FILTERS[2], [115, -52]);
        assert_eq!(ADPCM_FILTERS[3], [98, -55]);
        assert_eq!(ADPCM_FILTERS[4], [122, -60]);
    }

    #[test]
    fn test_decode_silence() {
        let mut decoder = AdpcmDecoder::new();
        let silence_block = [0u8; 16];
        let samples = decoder.decode_block(&silence_block).unwrap();
        assert_eq!(samples.len(), 28);
        assert!(samples.iter().all(|&s| s == 0));
    }

    #[test]
    fn the_residuals_are_sign_extended_nibbles_shifted_right() {
        let mut nibbles = nibbles_of(0);
        nibbles[0] = 7;
        nibbles[1] = 8;
        nibbles[2] = 0xF;
        let sample = decode_adpcm(&adpcm_block(0, 0, AdpcmFlags::END, &nibbles));
        assert_eq!(sample.pcm.len(), ADPCM_SAMPLES_PER_BLOCK);
        assert_eq!(sample.pcm[0], 28672);
        assert_eq!(sample.pcm[1], -32768);
        assert_eq!(sample.pcm[2], -4096);
        assert_eq!(sample.pcm[3], 0);

        let mut nibbles = nibbles_of(1);
        nibbles[1] = 0xF;
        let sample = decode_adpcm(&adpcm_block(0, 12, 0, &nibbles));
        assert_eq!(sample.pcm[0], 1);
        assert_eq!(sample.pcm[1], -1);
        assert_eq!(sample.reserved_blocks, 0);

        for shift in [13u8, 14, 15] {
            let sample = decode_adpcm(&adpcm_block(0, shift, 0, &nibbles_of(1)));
            assert_eq!(sample.pcm[0], 8);
            assert_eq!(sample.reserved_blocks, 1);
        }

        let mut nibbles = nibbles_of(0);
        nibbles[0] = 2;
        nibbles[1] = 5;
        let sample = decode_adpcm(&adpcm_block(0, 12, 0, &nibbles));
        assert_eq!(sample.pcm[0], 2);
        assert_eq!(sample.pcm[1], 5);
    }

    #[test]
    fn each_filter_predicts_from_the_last_two_samples() {
        // The first block leaves 4096 in both history samples; the second has zero residuals, so it shows the
        // prediction alone: (s1 * k0 + s2 * k1 + 32) >> 6.
        let first = adpcm_block(0, 0, 0, &nibbles_of(1));
        let decode = |filter: u8| {
            let mut bytes = first.clone();
            bytes.extend(adpcm_block(filter, 0, 0, &nibbles_of(0)));
            decode_adpcm(&bytes).pcm[ADPCM_SAMPLES_PER_BLOCK]
        };
        assert_eq!(decode(0), 0);
        assert_eq!(decode(1), 3840);
        assert_eq!(decode(2), 4032);
        assert_eq!(decode(3), 2752);
        assert_eq!(decode(4), 3968);
        // Reserved filters decode as filter 4.
        assert_eq!(decode(5), 3968);
        assert_eq!(decode(7), 3968);

        let mut bytes = first.clone();
        bytes.extend(adpcm_block(5, 0, 0, &nibbles_of(0)));
        assert_eq!(decode_adpcm(&bytes).reserved_blocks, 1);

        let mut bytes = first.clone();
        bytes.extend(adpcm_block(1, 0, 0, &nibbles_of(0)));
        let sample = decode_adpcm(&bytes);
        assert_eq!(sample.pcm[ADPCM_SAMPLES_PER_BLOCK + 1], 3600);
    }

    #[test]
    fn the_result_is_clamped_to_16_bits() {
        let mut bytes = adpcm_block(0, 0, 0, &nibbles_of(7));
        bytes.extend(adpcm_block(1, 0, 0, &nibbles_of(7)));
        assert_eq!(decode_adpcm(&bytes).pcm.last(), Some(&32767));

        let mut bytes = adpcm_block(0, 0, 0, &nibbles_of(8));
        bytes.extend(adpcm_block(1, 0, 0, &nibbles_of(8)));
        assert_eq!(decode_adpcm(&bytes).pcm.last(), Some(&-32768));
    }

    #[test]
    fn the_flags_end_the_sample_and_mark_its_loop() {
        let sample = decode_adpcm(&adpcm_sample(&[0, 0, AdpcmFlags::END, 0x07]));
        assert!(sample.terminated);
        assert_eq!(sample.blocks, 3);
        assert_eq!(sample.pcm.len(), 3 * ADPCM_SAMPLES_PER_BLOCK);
        assert!(sample.loop_info.is_none());

        let sample = decode_adpcm(&adpcm_sample(&[
            0,
            AdpcmFlags::LOOP_START,
            0,
            AdpcmFlags::END | AdpcmFlags::REPEAT,
        ]));
        assert_eq!(
            sample.loop_info,
            Some(AdpcmLoop {
                start: 28,
                end: 112
            })
        );
        assert!(!sample.loop_start_implied);

        let sample = decode_adpcm(&adpcm_sample(&[
            AdpcmFlags::LOOP_START,
            AdpcmFlags::LOOP_START | AdpcmFlags::REPEAT,
            AdpcmFlags::END | AdpcmFlags::REPEAT,
        ]));
        assert_eq!(
            sample.loop_info,
            Some(AdpcmLoop {
                start: 28,
                end: 84
            })
        );

        let sample = decode_adpcm(&adpcm_sample(&[0, AdpcmFlags::END | AdpcmFlags::REPEAT]));
        assert_eq!(
            sample.loop_info,
            Some(AdpcmLoop {
                start: 0,
                end: 56
            })
        );
        assert!(sample.loop_start_implied);

        let mut bytes = adpcm_sample(&[0, 0]);
        bytes.resize(bytes.len() + 5, 0);
        let sample = decode_adpcm(&bytes);
        assert!(!sample.terminated);
        assert_eq!(sample.blocks, 2);
        assert_eq!(sample.trailing_bytes, 5);

        let sample = decode_adpcm(&[]);
        assert!(sample.pcm.is_empty());
        assert!(!sample.terminated);

        let mut block = adpcm_block(0, 12, AdpcmFlags::END, &nibbles_of(0));
        block[0] |= 0x80;
        assert_eq!(decode_adpcm(&block).reserved_blocks, 1);
    }
}
