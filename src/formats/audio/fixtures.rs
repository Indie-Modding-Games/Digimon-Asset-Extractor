//! Builders of made-up sound data for the tests. Nothing here comes from a disc.

use crate::formats::adpcm::{AdpcmFlags, ADPCM_BLOCK_SIZE, ADPCM_SAMPLES_PER_BLOCK};
use crate::formats::vab2::{VAB_PROGRAM_SLOTS, VAB_TONES_PER_PROGRAM, VAB_VAG_SLOTS};

/// The bytes of a file, built the way the tests of the reference build them.
pub struct Bytes(Vec<u8>);

impl Bytes {
    pub fn new() -> Self {
        Bytes(Vec::new())
    }

    pub fn u8(&mut self, value: u8) -> &mut Self {
        self.0.push(value);
        self
    }

    pub fn le16(&mut self, value: u16) -> &mut Self {
        self.u8(value as u8);
        self.u8((value >> 8) as u8)
    }

    pub fn le32(&mut self, value: u32) -> &mut Self {
        self.le16(value as u16);
        self.le16((value >> 16) as u16)
    }

    pub fn text(&mut self, text: &str) -> &mut Self {
        for byte in text.bytes() {
            self.u8(byte);
        }
        self
    }

    pub fn fill(&mut self, count: usize, value: u8) -> &mut Self {
        for _ in 0..count {
            self.u8(value);
        }
        self
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for Bytes {
    fn default() -> Self {
        Bytes::new()
    }
}

/// One ADPCM block: the header (filter << 4 | shift), the flags, then 28 nibbles.
pub fn adpcm_block(filter: u8, shift: u8, flags: u8, nibbles: &[u8]) -> Vec<u8> {
    let mut block = vec![0u8; ADPCM_BLOCK_SIZE];
    block[0] = ((filter & 0x0F) << 4) | (shift & 0x0F);
    block[1] = flags;
    for position in 0..ADPCM_SAMPLES_PER_BLOCK {
        block[2 + position / 2] |= (nibbles[position] & 0x0F) << (4 * (position % 2));
    }
    block
}

/// A sample of blocks of constant nibbles, the flags given per block.
pub fn adpcm_sample(flags: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for flag in flags {
        bytes.extend(adpcm_block(0, 12, *flag, &vec![1u8; ADPCM_SAMPLES_PER_BLOCK]));
    }
    bytes
}

/// A tone of a synthetic VAB.
#[derive(Debug, Clone, Copy)]
pub struct ToneSpec {
    pub min: u8,
    pub max: u8,
    pub center: u8,
    pub shift: u8,
    pub vol: u8,
    pub pan: u8,
    pub mode: u8,
    pub adsr1: u16,
    pub adsr2: u16,
    pub vag: i16,
}

impl Default for ToneSpec {
    fn default() -> Self {
        Self {
            min: 60,
            max: 60,
            center: 60,
            shift: 0,
            vol: 127,
            pan: 64,
            mode: 0,
            adsr1: 0x80FF,
            adsr2: 0x5FC0,
            vag: 1,
        }
    }
}

/// A program of a synthetic VAB, in its slot.
#[derive(Debug, Clone, Default)]
pub struct ProgramSpec {
    pub slot: usize,
    pub tones: Vec<ToneSpec>,
    pub mvol: u8,
    pub mpan: u8,
}

/// Builds a VAB (version 7) and returns it; the vag sizes are the byte sizes of the samples 1 and up, divided by 8 in
/// the table.
pub fn build_vab(programs: &[ProgramSpec], vag_sizes: &[usize], vs: u16) -> Bytes {
    let vh_size = crate::formats::vab2::vab_header_size(programs.len());
    let vb_size: usize = vag_sizes.iter().sum();
    let tone_count: usize = programs.iter().map(|program| program.tones.len()).sum();

    let mut b = Bytes::new();
    b.text("pBAV")
        .le32(7) // version
        .le32(0) // id
        .le32((vh_size + vb_size) as u32) // fsize: the VH and the VB
        .le16(0) // reserved0
        .le16(programs.len() as u16)
        .le16(tone_count as u16)
        .le16(vs)
        .u8(127) // mvol
        .u8(64) // pan
        .u8(0) // attr1
        .u8(0) // attr2
        .le32(0); // reserved1

    for slot in 0..VAB_PROGRAM_SLOTS {
        let program = programs.iter().find(|program| program.slot == slot);
        match program {
            Some(program) => {
                b.u8(program.tones.len() as u8)
                    .u8(program.mvol)
                    .u8(0) // prior
                    .u8(0xFF) // mode
                    .u8(program.mpan)
                    .u8(0) // reserved0
                    .le16(0) // attr
                    .le32(0) // reserved1
                    .le32(0); // reserved2
            }
            None => {
                b.u8(0)
                    .u8(127)
                    .u8(0)
                    .u8(0xFF)
                    .u8(64)
                    .u8(0)
                    .le16(0)
                    .le32(0)
                    .le32(0);
            }
        }
    }

    for program in programs {
        for index in 0..VAB_TONES_PER_PROGRAM {
            if index >= program.tones.len() {
                b.fill(32, 0);
                continue;
            }
            let tone = program.tones[index];
            b.u8(0) // prior
                .u8(tone.mode)
                .u8(tone.vol)
                .u8(tone.pan)
                .u8(tone.center)
                .u8(tone.shift)
                .u8(tone.min)
                .u8(tone.max)
                .u8(0) // vibW
                .u8(0) // vibT
                .u8(0) // porW
                .u8(0) // porT
                .u8(0) // pbmin
                .u8(0) // pbmax
                .u8(0) // reserved1
                .u8(0) // reserved2
                .le16(tone.adsr1)
                .le16(tone.adsr2)
                .le16(program.slot as u16)
                .le16(tone.vag as u16)
                .fill(8, 0);
        }
    }

    b.le16(0);
    for index in 1..VAB_VAG_SLOTS {
        let size = vag_sizes.get(index - 1).map(|size| *size / 8).unwrap_or(0);
        b.le16(size as u16);
    }

    b
}

/// The flags of a block that ends the sample, and one that repeats it.
pub const END: u8 = AdpcmFlags::END;
/// The flags of a block that repeats the sample from its loop start.
pub const REPEAT: u8 = AdpcmFlags::REPEAT;
/// The flags of a block that the loop starts at.
pub const LOOP_START: u8 = AdpcmFlags::LOOP_START;
