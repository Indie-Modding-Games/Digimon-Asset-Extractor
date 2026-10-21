use crate::error::{DwExtractError, Result};
use std::io::{Read, Seek, SeekFrom};

/// The magic of a VAB: "VABp" stored as a little-endian u32.
pub const VAB_MAGIC: &[u8] = b"pBAV";
/// The size of the header of a VAB, the magic included.
pub const VAB_HEADER_SIZE: usize = 0x20;
pub const VAB_PROGRAM_SLOTS: usize = 128;
pub const VAB_TONES_PER_PROGRAM: usize = 16;
pub const VAB_VAG_SLOTS: usize = 256;
/// The size of the header of a sound bank: three offsets.
pub const BANK_HEADER_SIZE: u32 = 12;
/// The size of the header of a music font: four offsets.
pub const FONT_HEADER_SIZE: u32 = 16;
/// The unit of the size table of the samples: the size is in eighths of a byte.
pub const VAG_SIZE_UNIT: usize = 8;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VabHeader {
    pub version: u32,
    pub id: u32,
    pub fsize: u32,
    pub reserved0: u16,
    pub ps: u16,
    pub ts: u16,
    pub vs: u16,
    pub mvol: u8,
    pub pan: u8,
    pub attr1: u8,
    pub attr2: u8,
    pub reserved1: u32,
}

impl VabHeader {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; VAB_HEADER_SIZE];
        reader.read_exact(&mut buf)?;
        if &buf[0..VAB_MAGIC.len()] != VAB_MAGIC {
            return Err(DwExtractError::InvalidFormat(
                "not a VAB header: it does not start with pBAV".into(),
            ));
        }
        Ok(Self {
            version: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            id: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            fsize: u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            reserved0: u16::from_le_bytes([buf[16], buf[17]]),
            ps: u16::from_le_bytes([buf[18], buf[19]]),
            ts: u16::from_le_bytes([buf[20], buf[21]]),
            vs: u16::from_le_bytes([buf[22], buf[23]]),
            mvol: buf[24],
            pan: buf[25],
            attr1: buf[26],
            attr2: buf[27],
            reserved1: u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]),
        })
    }

    /// The size of the header of a VAB on its own.
    pub fn size(&self) -> usize {
        VAB_HEADER_SIZE
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ProgramAttributes {
    pub tones: u8,
    pub mvol: u8,
    pub prior: u8,
    pub mode: u8,
    pub mpan: u8,
    pub reserved0: u8,
    pub attr: i16,
    pub reserved1: u32,
    pub reserved2: u32,
}

impl ProgramAttributes {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 16];
        reader.read_exact(&mut buf)?;
        Ok(Self {
            tones: buf[0],
            mvol: buf[1],
            prior: buf[2],
            mode: buf[3],
            mpan: buf[4],
            reserved0: buf[5],
            attr: i16::from_le_bytes([buf[6], buf[7]]),
            reserved1: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            reserved2: u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.tones == 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ToneAttributes {
    pub prior: u8,
    pub mode: u8,
    pub vol: u8,
    pub pan: u8,
    pub center: u8,
    pub shift: u8,
    pub min: u8,
    pub max: u8,
    pub vib_w: u8,
    pub vib_t: u8,
    pub por_w: u8,
    pub por_t: u8,
    pub pbmin: u8,
    pub pbmax: u8,
    pub reserved1: u8,
    pub reserved2: u8,
    pub adsr1: u16,
    pub adsr2: u16,
    pub prog: i16,
    pub vag: i16,
    pub reserved: [i16; 4],
}

impl ToneAttributes {
    pub const REVERB_MODE: u8 = 0x04;

    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;
        Ok(Self {
            prior: buf[0],
            mode: buf[1],
            vol: buf[2],
            pan: buf[3],
            center: buf[4],
            shift: buf[5],
            min: buf[6],
            max: buf[7],
            vib_w: buf[8],
            vib_t: buf[9],
            por_w: buf[10],
            por_t: buf[11],
            pbmin: buf[12],
            pbmax: buf[13],
            reserved1: buf[14],
            reserved2: buf[15],
            adsr1: u16::from_le_bytes([buf[16], buf[17]]),
            adsr2: u16::from_le_bytes([buf[18], buf[19]]),
            prog: i16::from_le_bytes([buf[20], buf[21]]),
            vag: i16::from_le_bytes([buf[22], buf[23]]),
            reserved: [
                i16::from_le_bytes([buf[24], buf[25]]),
                i16::from_le_bytes([buf[26], buf[27]]),
                i16::from_le_bytes([buf[28], buf[29]]),
                i16::from_le_bytes([buf[30], buf[31]]),
            ],
        })
    }

    pub fn uses_reverb(&self) -> bool {
        (self.mode & Self::REVERB_MODE) != 0
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToneBlock {
    pub program: usize,
    pub tones: [ToneAttributes; VAB_TONES_PER_PROGRAM],
}

impl ToneBlock {
    pub fn read<R: Read>(reader: &mut R, program: usize) -> Result<Self> {
        let mut tones = [ToneAttributes::default(); VAB_TONES_PER_PROGRAM];
        for tone in &mut tones {
            *tone = ToneAttributes::read(reader)?;
        }
        Ok(Self { program, tones })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vab {
    pub header: VabHeader,
    pub programs: [ProgramAttributes; VAB_PROGRAM_SLOTS],
    pub tone_blocks: Vec<ToneBlock>,
    pub vag_sizes: [u16; VAB_VAG_SLOTS],
}

impl Vab {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let header = VabHeader::read(reader)?;

        if header.vs as usize >= VAB_VAG_SLOTS {
            return Err(DwExtractError::InvalidFormat(format!(
                "VAB has {} samples; the size table holds {}",
                header.vs,
                VAB_VAG_SLOTS - 1
            )));
        }

        let size = vab_header_size(header.ps as usize);
        if (header.fsize as usize) < size {
            return Err(DwExtractError::InvalidFormat(format!(
                "VAB fsize {} is smaller than its own header ({} bytes)",
                header.fsize, size
            )));
        }

        let mut programs = [ProgramAttributes::default(); VAB_PROGRAM_SLOTS];
        for prog in &mut programs {
            *prog = ProgramAttributes::read(reader)?;
        }

        // The VH stores a block of tones for each program that has tones, in program order, not one per program slot.
        let used: Vec<usize> = programs
            .iter()
            .enumerate()
            .filter(|(_, prog)| !prog.is_empty())
            .map(|(index, _)| index)
            .collect();
        if used.len() != header.ps as usize {
            return Err(DwExtractError::InvalidFormat(format!(
                "VAB header says {} programs, but {} program slots have tones",
                header.ps,
                used.len()
            )));
        }

        let mut tone_blocks = Vec::with_capacity(used.len());
        for program in used {
            tone_blocks.push(ToneBlock::read(reader, program)?);
        }

        let mut vag_sizes = [0u16; VAB_VAG_SLOTS];
        for size in &mut vag_sizes {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            *size = u16::from_le_bytes(buf);
        }

        let vab = Self {
            header,
            programs,
            tone_blocks,
            vag_sizes,
        };
        vab.check_tones()?;
        Ok(vab)
    }

    /// Every used tone must name the program it is in and a sample of the size table.
    fn check_tones(&self) -> Result<()> {
        for block in &self.tone_blocks {
            let program = &self.programs[block.program];
            if program.tones as usize > VAB_TONES_PER_PROGRAM {
                return Err(DwExtractError::InvalidFormat(format!(
                    "VAB program {} has {} tones; a program holds {}",
                    block.program,
                    program.tones,
                    VAB_TONES_PER_PROGRAM
                )));
            }
            for (index, tone) in block.tones.iter().take(program.tones as usize).enumerate() {
                if tone.prog as i32 != block.program as i32 {
                    return Err(DwExtractError::InvalidFormat(format!(
                        "VAB program {} tone {} says it belongs to program {}",
                        block.program, index, tone.prog
                    )));
                }
                if tone.vag < 1
                    || tone.vag as usize > self.header.vs as usize
                    || self.vag_sizes[tone.vag as usize] == 0
                {
                    return Err(DwExtractError::InvalidFormat(format!(
                        "VAB program {} tone {} plays sample {}, which is not one of the {}",
                        block.program, index, tone.vag, self.header.vs
                    )));
                }
            }
        }
        Ok(())
    }

    /// The size of the VH in bytes.
    pub fn size(&self) -> usize {
        vab_header_size(self.tone_blocks.len())
    }

    /// The size of the VB: the fsize of the header minus the size of the VH.
    pub fn vb_size(&self) -> usize {
        (self.header.fsize as usize).saturating_sub(self.size())
    }

    /// The tones of a program, or nothing for an empty slot.
    pub fn tones_of(&self, program: usize) -> Option<&[ToneAttributes]> {
        if program >= VAB_PROGRAM_SLOTS {
            return None;
        }
        let block = self.tone_blocks.iter().find(|block| block.program == program)?;
        let count = (self.programs[program].tones as usize).min(VAB_TONES_PER_PROGRAM);
        Some(&block.tones[..count])
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VhbHeader {
    pub vh_offset: u32,
    pub vb_offset: u32,
    /// Where the VB ends: the third offset of the header.
    pub vb_end: u32,
    /// Where the SEQs end: the same as `vb_end` in a sound bank, the fourth offset in a music font.
    pub end: u32,
    pub music_font: bool,
}

impl VhbHeader {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 12];
        reader.read_exact(&mut buf)?;
        let vh_offset = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let vb_offset = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let third = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);

        if vh_offset == 0 || vb_offset == 0 || third == 0 {
            return Err(DwExtractError::InvalidFormat(
                "VHB header is shorter than its three offsets".into(),
            ));
        }

        // A music font starts its VH at 16 bytes, a sound bank at 12.
        let music_font = vh_offset == FONT_HEADER_SIZE;
        if vh_offset != BANK_HEADER_SIZE && !music_font {
            return Err(DwExtractError::InvalidFormat(format!(
                "VHB header says the VH starts at {}; it is 12 or 16",
                vh_offset
            )));
        }

        // The third offset is where the VB ends; a music font keeps the SEQs after it.
        let end = if music_font {
            let mut buf = [0u8; 4];
            reader.seek(SeekFrom::Start(12))?;
            reader.read_exact(&mut buf)?;
            let seq_end = u32::from_le_bytes(buf);
            if seq_end == 0 || seq_end < third {
                return Err(DwExtractError::InvalidFormat(
                    "VHB music font's SEQs end before the VB does".into(),
                ));
            }
            seq_end
        } else {
            third
        };

        Ok(Self {
            vh_offset,
            vb_offset,
            vb_end: third,
            end,
            music_font,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Vhb {
    pub header: VhbHeader,
    pub vab: Vab,
    pub vb_data: Vec<u8>,
    pub seq_data: Vec<u8>,
}

impl Vhb {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let length = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let header = VhbHeader::read(reader)?;

        if header.vb_offset as u64 > length {
            return Err(DwExtractError::InvalidFormat(format!(
                "VHB VB offset {} is outside a file of {} bytes",
                header.vb_offset, length
            )));
        }

        reader.seek(SeekFrom::Start(header.vh_offset as u64))?;
        let vab = Vab::read(reader)?;

        let vb_size = vab.vb_size();
        if vb_size as u64 > length - header.vb_offset as u64 {
            return Err(DwExtractError::InvalidFormat(format!(
                "VHB VB of {} bytes at {} runs past a file of {} bytes",
                vb_size, header.vb_offset, length
            )));
        }
        let vb_end = header.vb_offset as u64 + vb_size as u64;

        reader.seek(SeekFrom::Start(header.vb_offset as u64))?;
        let mut vb_data = vec![0u8; vb_size];
        reader.read_exact(&mut vb_data)?;

        // The third offset of the header is where the VB ends.
        if header.vb_end as u64 != vb_end {
            return Err(DwExtractError::InvalidFormat(format!(
                "VHB says the {} ends at {}; the VB ends at {}",
                if header.music_font { "VB" } else { "bank" },
                header.vb_end,
                vb_end
            )));
        }

        let mut seq_data = Vec::new();
        if header.music_font {
            if header.end as u64 > length {
                return Err(DwExtractError::InvalidFormat(format!(
                    "VHB music font's SEQs end outside a file of {} bytes",
                    length
                )));
            }
            let seq_size = header.end as u64 - vb_end;
            if seq_size > 0 {
                reader.seek(SeekFrom::Start(vb_end))?;
                seq_data = vec![0u8; seq_size as usize];
                reader.read_exact(&mut seq_data)?;
            }
        }

        Ok(Self {
            header,
            vab,
            vb_data,
            seq_data,
        })
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = std::io::Cursor::new(data);
        Self::read(&mut cursor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VagLocation {
    pub number: usize,
    pub offset: usize,
    pub size: usize,
    pub beyond_count: bool,
}

/// Splits the VB into its samples: sample i starts after samples 1..i-1. The samples the header counts must fit in
/// the VB; the ones the size table lists after them are returned too, marked, while they fit.
pub fn locate_vags(vab: &Vab, vb_size: usize) -> Result<Vec<VagLocation>> {
    let vs = vab.header.vs as usize;
    let mut locations = Vec::new();
    let mut offset = 0usize;

    for number in 1..VAB_VAG_SLOTS {
        let size = (vab.vag_sizes[number] as usize) * VAG_SIZE_UNIT;
        let beyond_count = number > vs;
        if size == 0 {
            continue;
        }
        if size > vb_size.saturating_sub(offset.min(vb_size)) {
            if beyond_count {
                break;
            }
            return Err(DwExtractError::InvalidFormat(format!(
                "VAB sample {} ({} bytes at {}) does not fit in a VB of {} bytes",
                number, size, offset, vb_size
            )));
        }
        locations.push(VagLocation {
            number,
            offset,
            size,
            beyond_count,
        });
        offset += size;
    }

    Ok(locations)
}

pub fn vab_header_size(programs: usize) -> usize {
    VAB_HEADER_SIZE
        + VAB_PROGRAM_SLOTS * 16
        + programs * VAB_TONES_PER_PROGRAM * 32
        + VAB_VAG_SLOTS * 2
}

pub fn read_vab<R: Read + Seek>(reader: &mut R) -> Result<Vab> {
    Vab::read(reader)
}

pub fn read_vhb<R: Read + Seek>(reader: &mut R) -> Result<Vhb> {
    Vhb::read(reader)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes of a file, built the way the tests of the reference build them.
    struct Bytes(Vec<u8>);

    impl Bytes {
        fn new() -> Self {
            Bytes(Vec::new())
        }

        fn u8(&mut self, value: u8) -> &mut Self {
            self.0.push(value);
            self
        }

        fn le16(&mut self, value: u16) -> &mut Self {
            self.u8(value as u8);
            self.u8((value >> 8) as u8)
        }

        fn le32(&mut self, value: u32) -> &mut Self {
            self.le16(value as u16);
            self.le16((value >> 16) as u16)
        }

        fn text(&mut self, text: &str) -> &mut Self {
            for byte in text.bytes() {
                self.u8(byte);
            }
            self
        }

        fn fill(&mut self, count: usize, value: u8) -> &mut Self {
            for _ in 0..count {
                self.u8(value);
            }
            self
        }

        fn bytes(&self) -> &[u8] {
            &self.0
        }
    }

    /// A VAB with one program that has one tone, and a size table.
    struct VabBuilder {
        /// The program count of the header, which can disagree with the slots that have tones.
        ps: u16,
        vs: u16,
        /// The program whose slot has tones.
        program: u16,
        /// The program the tone names.
        tone_program: i16,
        /// The sample the tone plays.
        tone_vag: i16,
        /// The size table: the number of the sample and its size in eighths of a byte.
        vag_sizes: Vec<(usize, u16)>,
        vb_size: usize,
    }

    impl Default for VabBuilder {
        fn default() -> Self {
            Self {
                ps: 1,
                vs: 1,
                program: 5,
                tone_program: 5,
                tone_vag: 1,
                vag_sizes: vec![(1, 2)],
                vb_size: 16,
            }
        }
    }

    impl VabBuilder {
        fn build(&self) -> Bytes {
            let mut b = Bytes::new();
            b.text("pBAV")
                .le32(7) // version
                .le32(0) // id
                .le32((vab_header_size(1) + self.vb_size) as u32) // fsize: the VH and the VB
                .le16(0) // reserved0
                .le16(self.ps)
                .le16(1) // ts: one tone
                .le16(self.vs)
                .u8(0) // mvol
                .u8(0) // pan
                .u8(0) // attr1
                .u8(0) // attr2
                .le32(0); // reserved1

            for index in 0..VAB_PROGRAM_SLOTS {
                if index == self.program as usize {
                    b.u8(1) // tones
                        .u8(0) // mvol
                        .u8(0) // prior
                        .u8(0) // mode
                        .u8(0) // mpan
                        .u8(0) // reserved0
                        .le16(0) // attr
                        .le32(0) // reserved1
                        .le32(0); // reserved2
                } else {
                    b.fill(16, 0);
                }
            }

            b.u8(0) // prior
                .u8(0) // mode
                .u8(100) // vol
                .u8(64) // pan
                .u8(60) // center
                .u8(0) // shift
                .u8(0) // min
                .u8(127) // max
                .u8(0) // vibW
                .u8(0) // vibT
                .u8(0) // porW
                .u8(0) // porT
                .u8(0) // pbmin
                .u8(2) // pbmax
                .u8(0) // reserved1
                .u8(0) // reserved2
                .le16(0) // adsr1
                .le16(0) // adsr2
                .le16(self.tone_program as u16)
                .le16(self.tone_vag as u16)
                .le16(0)
                .le16(0)
                .le16(0)
                .le16(0);
            b.fill(15 * 32, 0); // the fifteen empty tones of the block

            for index in 0..VAB_VAG_SLOTS {
                let size = self
                    .vag_sizes
                    .iter()
                    .find(|(number, _)| *number == index)
                    .map(|(_, size)| *size)
                    .unwrap_or(0);
                b.le16(size);
            }

            b
        }
    }

    /// A music font: the four offsets of the header, the VH, the VB and the bank of the sequences.
    fn music_font(vab: &Bytes, vb: &[u8], seq_bank: &[u8]) -> Bytes {
        let mut b = Bytes::new();
        let vh_offset = FONT_HEADER_SIZE as usize;
        let vb_offset = vh_offset + vab.bytes().len();
        let vb_end = vb_offset + vb.len();
        let seq_end = vb_end + seq_bank.len();
        b.le32(vh_offset as u32)
            .le32(vb_offset as u32)
            .le32(vb_end as u32)
            .le32(seq_end as u32);
        b.0.extend_from_slice(vab.bytes());
        b.0.extend_from_slice(vb);
        b.0.extend_from_slice(seq_bank);
        b
    }

    /// A sound bank: the three offsets of the header, the VH and the VB.
    fn sound_bank(vab: &Bytes, vb: &[u8]) -> Bytes {
        let mut b = Bytes::new();
        let vh_offset = BANK_HEADER_SIZE as usize;
        let vb_offset = vh_offset + vab.bytes().len();
        let vb_end = vb_offset + vb.len();
        b.le32(vh_offset as u32).le32(vb_offset as u32).le32(vb_end as u32);
        b.0.extend_from_slice(vab.bytes());
        b.0.extend_from_slice(vb);
        b
    }

    /// The bank of a sequence: the table of the offsets and the sequence itself.
    fn seq_bank(seq: &[u8]) -> Bytes {
        let mut b = Bytes::new();
        b.le32(4).le32(4);
        b.0.extend_from_slice(seq);
        b
    }

    #[test]
    fn test_vab_header_size() {
        assert_eq!(vab_header_size(0), 32 + 128 * 16 + 256 * 2);
        assert_eq!(vab_header_size(1), 32 + 128 * 16 + 1 * 16 * 32 + 256 * 2);
    }

    #[test]
    fn the_header_of_a_vab_starts_with_the_magic_and_the_version_after_it() {
        let vab = VabBuilder::default().build();
        let parsed = Vab::read(&mut std::io::Cursor::new(vab.bytes())).unwrap();
        assert_eq!(parsed.header.version, 7);
        assert_eq!(parsed.header.ps, 1);
        assert_eq!(parsed.header.ts, 1);
        assert_eq!(parsed.header.vs, 1);
        assert_eq!(parsed.header.fsize as usize, vab_header_size(1) + 16);
    }

    #[test]
    fn a_vab_without_the_magic_is_an_error() {
        let mut vab = VabBuilder::default().build();
        vab.0[0] = b'X';
        assert!(Vab::read(&mut std::io::Cursor::new(vab.bytes())).is_err());
    }

    #[test]
    fn the_tone_block_belongs_to_the_program_that_has_tones_and_not_to_the_slot_number() {
        let vab = VabBuilder::default().build();
        let parsed = Vab::read(&mut std::io::Cursor::new(vab.bytes())).unwrap();

        assert_eq!(parsed.tone_blocks.len(), 1);
        assert_eq!(parsed.tone_blocks[0].program, 5);
        assert_eq!(parsed.tones_of(5).unwrap().len(), 1);
        assert_eq!(parsed.tones_of(5).unwrap()[0].vag, 1);
        assert!(parsed.tones_of(0).is_none());
        assert!(parsed.tones_of(6).is_none());
    }

    #[test]
    fn a_vab_whose_program_count_does_not_match_its_slots_is_an_error() {
        let mut builder = VabBuilder::default();
        builder.ps = 2;
        let vab = builder.build();
        assert!(Vab::read(&mut std::io::Cursor::new(vab.bytes())).is_err());
    }

    #[test]
    fn a_tone_of_another_program_is_an_error() {
        let mut builder = VabBuilder::default();
        builder.tone_program = 6;
        let vab = builder.build();
        assert!(Vab::read(&mut std::io::Cursor::new(vab.bytes())).is_err());
    }

    #[test]
    fn a_tone_of_a_sample_that_is_not_in_the_bank_is_an_error() {
        let mut builder = VabBuilder::default();
        builder.tone_vag = 2;
        let vab = builder.build();
        assert!(Vab::read(&mut std::io::Cursor::new(vab.bytes())).is_err());
    }

    #[test]
    fn a_music_font_holds_its_samples_and_its_sequences() {
        let vab = VabBuilder::default().build();
        let vb = vec![0xAA; 16];
        let seq = seq_bank(&[1, 2, 3]);
        let font = music_font(&vab, &vb, seq.bytes());

        let parsed = Vhb::from_bytes(font.bytes()).unwrap();
        assert!(parsed.header.music_font);
        assert_eq!(parsed.header.vh_offset, FONT_HEADER_SIZE);
        assert_eq!(parsed.header.vb_end, 16 + vab.bytes().len() as u32 + 16);
        assert_eq!(parsed.vb_data, vb);
        assert_eq!(parsed.seq_data, seq.bytes());
        assert_eq!(parsed.vab.tone_blocks[0].program, 5);
        assert_eq!(parsed.vab.vb_size(), 16);
    }

    #[test]
    fn a_sound_bank_has_no_sequences() {
        let vab = VabBuilder::default().build();
        let vb = vec![0xBB; 16];
        let bank = sound_bank(&vab, &vb);

        let parsed = Vhb::from_bytes(bank.bytes()).unwrap();
        assert!(!parsed.header.music_font);
        assert!(parsed.seq_data.is_empty());
        assert_eq!(parsed.vb_data, vb);
    }

    #[test]
    fn a_vhb_whose_third_offset_is_not_the_end_of_the_vb_is_an_error() {
        let vab = VabBuilder::default().build();
        let vb = vec![0xAA; 16];
        let mut font = music_font(&vab, &vb, &[1, 2, 3]);
        font.0[8] = 0; // the third offset
        assert!(Vhb::from_bytes(font.bytes()).is_err());
    }

    #[test]
    fn the_samples_of_the_vb_are_located_one_after_the_other() {
        let mut builder = VabBuilder::default();
        builder.vs = 2;
        builder.vag_sizes = vec![(1, 2), (2, 3)]; // 16 and 24 bytes
        let vab = builder.build();
        let parsed = Vab::read(&mut std::io::Cursor::new(vab.bytes())).unwrap();

        let locations = locate_vags(&parsed, 40).unwrap();
        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0].number, 1);
        assert_eq!(locations[0].offset, 0);
        assert_eq!(locations[0].size, 16);
        assert!(!locations[0].beyond_count);
        assert_eq!(locations[1].offset, 16);
        assert_eq!(locations[1].size, 24);
    }

    #[test]
    fn a_sample_that_does_not_fit_in_the_vb_is_an_error() {
        let mut builder = VabBuilder::default();
        builder.vs = 1;
        builder.vag_sizes = vec![(1, 4)]; // 32 bytes
        let vab = builder.build();
        let parsed = Vab::read(&mut std::io::Cursor::new(vab.bytes())).unwrap();

        assert!(locate_vags(&parsed, 16).is_err());
        assert!(locate_vags(&parsed, 32).is_ok());
    }
}
