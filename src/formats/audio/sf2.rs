use crate::error::{DwExtractError, Result};
use std::collections::HashMap;
use std::io::Write;

const NAME_SIZE: usize = 20;
const LOOP_GUARD_POINTS: usize = 8;
const SAMPLE_PADDING: usize = 46;
const MONO_SAMPLE: u16 = 1;
const MAX_INDEX: usize = u16::MAX as usize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u16)]
pub enum Sf2Generator {
    /// The generator of the terminal record of a list of generators.
    StartAddrsOffset = 0,
    #[default]
    ReverbEffectsSend = 16,
    Pan = 17,
    AttackVolEnv = 34,
    HoldVolEnv = 35,
    DecayVolEnv = 36,
    SustainVolEnv = 37,
    ReleaseVolEnv = 38,
    Instrument = 41,
    KeyRange = 43,
    InitialAttenuation = 48,
    FineTune = 52,
    SampleId = 53,
    SampleModes = 54,
    OverridingRootKey = 58,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sf2GeneratorValue {
    pub generator: Sf2Generator,
    pub amount: i16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SampleLoop {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SamplePlace {
    pub start: u32,
    pub end: u32,
    pub loop_start: u32,
    pub loop_end: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sf2Sample {
    pub name: String,
    pub pcm: Vec<i16>,
    pub sample_rate: u32,
    pub loop_info: Option<SampleLoop>,
    pub original_pitch: u8,
    pub pitch_correction: i8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sf2Zone {
    pub key_low: u8,
    pub key_high: u8,
    pub generators: Vec<Sf2GeneratorValue>,
    pub sample: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sf2Instrument {
    pub name: String,
    pub zones: Vec<Sf2Zone>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sf2Preset {
    pub name: String,
    pub preset: u16,
    pub bank: u16,
    pub instrument: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoundFont {
    pub name: String,
    pub samples: Vec<Sf2Sample>,
    pub instruments: Vec<Sf2Instrument>,
    pub presets: Vec<Sf2Preset>,
}

struct RiffWriter {
    data: Vec<u8>,
    chunk_starts: Vec<usize>,
}

impl RiffWriter {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            chunk_starts: Vec::new(),
        }
    }

    fn begin_list(&mut self, list_type: &str, list_subtype: &str) -> usize {
        let start = self.data.len();
        self.data.extend_from_slice(list_type.as_bytes());
        self.data.extend_from_slice(&0u32.to_le_bytes()); // placeholder size
        self.data.extend_from_slice(list_subtype.as_bytes());
        self.chunk_starts.push(start);
        start
    }

    fn begin_chunk(&mut self, chunk_id: &str) -> usize {
        let start = self.data.len();
        self.data.extend_from_slice(chunk_id.as_bytes());
        self.data.extend_from_slice(&0u32.to_le_bytes()); // placeholder size
        self.chunk_starts.push(start);
        start
    }

    fn end_chunk(&mut self, start: usize) {
        let size = (self.data.len() - start - 8) as u32;
        let size_bytes = size.to_le_bytes();
        self.data[start + 4..start + 8].copy_from_slice(&size_bytes);
        // The chunks of a RIFF file have an even size.
        if size % 2 != 0 {
            self.data.push(0);
        }
    }

    fn u8(&mut self, value: u8) {
        self.data.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes text into a field of the given size, cut to leave at least one NUL and padded with NULs.
    fn name(&mut self, name: &str, size: usize) {
        let bytes = name.as_bytes();
        let length = bytes.len().min(size.saturating_sub(1));
        self.data.extend_from_slice(&bytes[..length]);
        for _ in length..size {
            self.data.push(0);
        }
    }

    fn pcm_chunk(&mut self, chunk_id: &str, points: &[i16]) {
        let start = self.begin_chunk(chunk_id);
        for &p in points {
            self.data.extend_from_slice(&p.to_le_bytes());
        }
        let chunk_start = self.chunk_starts.pop().unwrap();
        self.end_chunk(chunk_start);
    }

    fn take(mut self) -> Vec<u8> {
        // Update RIFF size
        let total_size = (self.data.len() - 8) as u32;
        self.data[4..8].copy_from_slice(&total_size.to_le_bytes());
        self.data
    }
}

fn check_font(font: &SoundFont) -> Result<()> {
    for (i, sample) in font.samples.iter().enumerate() {
        if let Some(loop_info) = &sample.loop_info {
            if loop_info.start >= loop_info.end || loop_info.end > sample.pcm.len() as u32 {
                return Err(DwExtractError::InvalidFormat(format!(
                    "SF2 sample {}: loop {}-{} is outside its {} points", 
                    sample.name, loop_info.start, loop_info.end, sample.pcm.len()
                )));
            }
        }
    }

    let mut zones = 0;
    let mut generators = 0;
    for instrument in &font.instruments {
        for zone in &instrument.zones {
            if zone.sample >= font.samples.len() {
                return Err(DwExtractError::InvalidFormat(format!(
                    "SF2 instrument {} plays sample {}; there are {}", 
                    instrument.name, zone.sample, font.samples.len()
                )));
            }
            if zone.key_low > zone.key_high || zone.key_high > 127 {
                return Err(DwExtractError::InvalidFormat(format!(
                    "SF2 instrument {} has the key range {}-{}", 
                    instrument.name, zone.key_low, zone.key_high
                )));
            }
            if zone.generators.iter().any(|v| is_placed_by_writer(v.generator)) {
                return Err(DwExtractError::InvalidFormat(format!(
                    "SF2 instrument {} gives a generator the writer places itself", 
                    instrument.name
                )));
            }
            zones += 1;
            generators += zone.generators.len() + 2;
        }
    }

    for preset in &font.presets {
        if preset.instrument >= font.instruments.len() {
            return Err(DwExtractError::InvalidFormat(format!(
                "SF2 preset {} plays instrument {}; there are {}", 
                preset.name, preset.instrument, font.instruments.len()
            )));
        }
    }

    if zones >= MAX_INDEX || generators >= MAX_INDEX || font.presets.len() >= MAX_INDEX || 
       font.instruments.len() >= MAX_INDEX {
        return Err(DwExtractError::InvalidFormat("SF2 tables outgrow their 16-bit indices".into()));
    }
    Ok(())
}

fn is_placed_by_writer(generator: Sf2Generator) -> bool {
    matches!(generator, Sf2Generator::KeyRange | Sf2Generator::SampleId | Sf2Generator::Instrument)
}

fn write_info_text(riff: &mut RiffWriter, id: &str, text: &str) {
    let start = riff.begin_chunk(id);
    let text_bytes = text.as_bytes();
    let len = if text_bytes.len() % 2 == 0 { text_bytes.len() + 2 } else { text_bytes.len() + 1 };
    let mut name_bytes = text.as_bytes().to_vec();
    name_bytes.resize(len, 0);
    riff.data.extend_from_slice(&name_bytes);
    riff.end_chunk(start);
}

fn write_sample_data(riff: &mut RiffWriter, font: &SoundFont) -> Result<Vec<SamplePlace>> {
    let mut places = Vec::new();
    let mut points = Vec::new();

    for sample in &font.samples {
        let mut place = SamplePlace::default();
        place.start = points.len() as u32;
        points.extend_from_slice(&sample.pcm);

        if let Some(loop_info) = &sample.loop_info {
            let length = loop_info.end - loop_info.start;
            for i in 0..8 {
                points.push(sample.pcm[(loop_info.start + (i % length) as u32) as usize]);
            }
            place.loop_start = place.start + loop_info.start;
            place.loop_end = place.start + loop_info.end;
        }
        place.end = points.len() as u32;
        if sample.loop_info.is_none() {
            place.loop_start = place.start;
            place.loop_end = place.end;
        }
        points.extend(std::iter::repeat(0).take(SAMPLE_PADDING));

        if points.len() > (u32::MAX / 2) as usize {
            return Err(DwExtractError::InvalidFormat("SF2 sample data outgrows a RIFF chunk".into()));
        }
        places.push(place);
    }

    let list_at = riff.begin_list("LIST", "sdta");
    riff.pcm_chunk("smpl", &points);
    riff.end_chunk(list_at);
    Ok(places)
}

fn write_bag(riff: &mut RiffWriter, generator: usize) {
    riff.u16(generator as u16);
    riff.u16(0);
}

fn write_generator(riff: &mut RiffWriter, generator: Sf2Generator, amount: u32) {
    riff.u16(generator as u16);
    riff.u16(amount as u16);
}

fn write_empty_modulators(riff: &mut RiffWriter, id: &str) {
    let start = riff.begin_chunk(id);
    for _ in 0..10 {
        riff.u8(0);
    }
    riff.end_chunk(start);
}

fn write_presets(riff: &mut RiffWriter, font: &SoundFont) {
    let start = riff.begin_chunk("phdr");
    for i in 0..=font.presets.len() {
        let terminal = i == font.presets.len();
        if terminal {
            riff.name("EOP", NAME_SIZE);
            riff.u16(0);
            riff.u16(0);
            riff.u16(i as u16);
        } else {
            let p = &font.presets[i];
            riff.name(&p.name, NAME_SIZE);
            riff.u16(p.preset);
            riff.u16(p.bank);
            riff.u16(i as u16);
        }
        riff.u32(0); // library
        riff.u32(0); // genre
        riff.u32(0); // morphology
    }
    riff.end_chunk(start);

    let start = riff.begin_chunk("pbag");
    for i in 0..=font.presets.len() {
        write_bag(riff, i);
    }
    riff.end_chunk(start);
    write_empty_modulators(riff, "pmod");

    let start = riff.begin_chunk("pgen");
    for preset in &font.presets {
        write_generator(riff, Sf2Generator::Instrument, preset.instrument as u32);
    }
    write_generator(riff, Sf2Generator::StartAddrsOffset, 0);
    riff.end_chunk(start);
}

fn write_instruments(riff: &mut RiffWriter, font: &SoundFont) {
    let mut bag = 0;
    let start = riff.begin_chunk("inst");
    for instrument in &font.instruments {
        riff.name(&instrument.name, NAME_SIZE);
        riff.u16(bag as u16);
        bag += instrument.zones.len();
    }
    riff.name("EOI", NAME_SIZE);
    riff.u16(bag as u16);
    riff.end_chunk(start);

    let mut generator = 0;
    let start = riff.begin_chunk("ibag");
    for instrument in &font.instruments {
        for zone in &instrument.zones {
            write_bag(riff, generator);
            generator += zone.generators.len() + 2;
        }
    }
    write_bag(riff, generator);
    riff.end_chunk(start);
    write_empty_modulators(riff, "imod");

    let start = riff.begin_chunk("igen");
    for instrument in &font.instruments {
        for zone in &instrument.zones {
            write_generator(riff, Sf2Generator::KeyRange, (zone.key_low as u32) | ((zone.key_high as u32) << 8));
            for value in &zone.generators {
                write_generator(riff, value.generator, value.amount as u32);
            }
            write_generator(riff, Sf2Generator::SampleId, zone.sample as u32);
        }
    }
    write_generator(riff, Sf2Generator::StartAddrsOffset, 0);
    riff.end_chunk(start);
}

fn write_sample_headers(riff: &mut RiffWriter, font: &SoundFont, places: &[SamplePlace]) {
    let start = riff.begin_chunk("shdr");
    for i in 0..=font.samples.len() {
        if i < font.samples.len() {
            let sample = &font.samples[i];
            let place = &places[i];
            riff.name(&sample.name, NAME_SIZE);
            riff.u32(place.start);
            riff.u32(place.end);
            riff.u32(place.loop_start);
            riff.u32(place.loop_end);
            riff.u32(sample.sample_rate);
            riff.u8(sample.original_pitch);
            riff.u8(sample.pitch_correction as u8);
            riff.u16(0);
            riff.u16(MONO_SAMPLE);
        } else {
            // The terminal record: the name and the rest of the 46 bytes of a record.
            riff.name("EOS", NAME_SIZE);
            for _ in 0..(46 - NAME_SIZE) {
                riff.u8(0);
            }
        }
    }
    riff.end_chunk(start);
}

pub fn write_sf2(font: &SoundFont) -> Result<Vec<u8>> {
    check_font(font)?;

    let mut riff = RiffWriter::new();
    let riff_start = riff.begin_list("RIFF", "sfbk");

    // INFO list
    let info_start = riff.begin_list("LIST", "INFO");
    let version_start = riff.begin_chunk("ifil");
    riff.u16(2);
    riff.u16(1);
    riff.end_chunk(version_start);
    write_info_text(&mut riff, "isng", "EMU8000");
    write_info_text(&mut riff, "INAM", &font.name);
    riff.end_chunk(info_start);

    // Sample data
    let places = write_sample_data(&mut riff, font)?;

    // Preset data
    let pdta_start = riff.begin_list("LIST", "pdta");
    write_presets(&mut riff, font);
    write_instruments(&mut riff, font);
    write_sample_headers(&mut riff, font, &places);
    riff.end_chunk(pdta_start);

    riff.end_chunk(riff_start);
    Ok(riff.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chunk of a RIFF file, read back the way the reference reads them.
    struct RiffChunk {
        id: String,
        /// The form type of the RIFF and LIST chunks.
        kind: String,
        data: Vec<u8>,
        children: Vec<RiffChunk>,
    }

    impl RiffChunk {
        fn child(&self, name: &str) -> Option<&RiffChunk> {
            self.children
                .iter()
                .find(|chunk| chunk.id == name || (chunk.id == "LIST" && chunk.kind == name))
        }
    }

    fn read_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    fn read_tag(bytes: &[u8], offset: usize) -> String {
        String::from_utf8_lossy(&bytes[offset..offset + 4]).to_string()
    }

    /// Reads the chunks that fill the bytes exactly, each padded to an even size.
    fn parse_chunks(bytes: &[u8]) -> Option<Vec<RiffChunk>> {
        let mut chunks = Vec::new();
        let mut offset = 0usize;
        while offset < bytes.len() {
            if bytes.len() - offset < 8 {
                return None;
            }
            let id = read_tag(bytes, offset);
            let size = read_u32(bytes, offset + 4) as usize;
            if size > bytes.len() - offset - 8 {
                return None;
            }
            let data = bytes[offset + 8..offset + 8 + size].to_vec();
            let mut children = Vec::new();
            let mut kind = String::new();
            if id == "RIFF" || id == "LIST" {
                if size < 4 {
                    return None;
                }
                kind = read_tag(&data, 0);
                children = parse_chunks(&data[4..])?;
            }
            offset += 8 + size + (size % 2);
            chunks.push(RiffChunk {
                id,
                kind,
                data,
                children,
            });
        }
        if offset != bytes.len() {
            return None;
        }
        Some(chunks)
    }

    /// Reads a whole RIFF file: one RIFF chunk that takes every byte.
    fn parse_riff(bytes: &[u8]) -> RiffChunk {
        let mut chunks = parse_chunks(bytes).unwrap_or_default();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, "RIFF");
        chunks.remove(0)
    }

    /// A font with two samples (the second looped), one instrument of two zones and two presets that play it.
    fn small_font() -> SoundFont {
        SoundFont {
            name: "test font".into(),
            samples: vec![
                Sf2Sample {
                    name: "one".into(),
                    pcm: vec![1, 2, 3],
                    sample_rate: 22050,
                    loop_info: None,
                    original_pitch: 60,
                    pitch_correction: 0,
                },
                Sf2Sample {
                    name: "two".into(),
                    pcm: vec![10, 20, 30, 40],
                    sample_rate: 44100,
                    loop_info: Some(SampleLoop { start: 1, end: 4 }),
                    original_pitch: 48,
                    pitch_correction: -3,
                },
            ],
            instruments: vec![Sf2Instrument {
                name: "instrument".into(),
                zones: vec![
                    Sf2Zone {
                        key_low: 0,
                        key_high: 59,
                        generators: vec![],
                        sample: 0,
                    },
                    Sf2Zone {
                        key_low: 60,
                        key_high: 127,
                        generators: vec![
                            Sf2GeneratorValue {
                                generator: Sf2Generator::FineTune,
                                amount: -25,
                            },
                            Sf2GeneratorValue {
                                generator: Sf2Generator::SampleModes,
                                amount: 1,
                            },
                        ],
                        sample: 1,
                    },
                ],
            }],
            presets: vec![
                Sf2Preset {
                    name: "first".into(),
                    preset: 0,
                    bank: 0,
                    instrument: 0,
                },
                Sf2Preset {
                    name: "second".into(),
                    preset: 5,
                    bank: 0,
                    instrument: 0,
                },
            ],
        }
    }

    #[test]
    fn test_sf2_generator_values() {
        assert_eq!(Sf2Generator::Pan as u16, 17);
        assert_eq!(Sf2Generator::SampleId as u16, 53);
    }

    #[test]
    fn test_is_placed_by_writer() {
        assert!(is_placed_by_writer(Sf2Generator::KeyRange));
        assert!(is_placed_by_writer(Sf2Generator::SampleId));
        assert!(is_placed_by_writer(Sf2Generator::Instrument));
        assert!(!is_placed_by_writer(Sf2Generator::Pan));
    }

    #[test]
    fn the_chunks_the_tables_and_the_terminal_records_of_a_soundfont() {
        let bytes = write_sf2(&small_font()).unwrap();
        assert_eq!(write_sf2(&small_font()).unwrap(), bytes);

        let riff = parse_riff(&bytes);
        assert_eq!(riff.id, "RIFF");
        assert_eq!(riff.kind, "sfbk");
        assert_eq!(riff.children.len(), 3);

        let info = riff.child("INFO").unwrap();
        assert_eq!(read_u16(info.child("ifil").unwrap().data.as_slice(), 0), 2);
        assert!(info.child("INAM").is_some());
        assert_eq!(info.child("INAM").unwrap().data.len() % 2, 0);

        let sample_data = riff.child("sdta").unwrap();
        let smpl = sample_data.child("smpl").unwrap();
        // 3 + 46 points, then 4 + 8 guard + 46.
        assert_eq!(smpl.data.len(), 2 * (3 + 46 + 4 + 8 + 46));

        let preset_data = riff.child("pdta").unwrap();
        let tables: [(&str, usize); 9] = [
            ("phdr", 38 * 3),
            ("pbag", 4 * 3),
            ("pmod", 10),
            ("pgen", 4 * 3),
            ("inst", 22 * 2),
            ("ibag", 4 * 3),
            ("imod", 10),
            ("igen", 4 * (2 + 4 + 1)),
            ("shdr", 46 * 3),
        ];
        assert_eq!(preset_data.children.len(), tables.len());
        for (index, (id, size)) in tables.iter().enumerate() {
            assert_eq!(preset_data.children[index].id, *id);
            assert_eq!(preset_data.children[index].data.len(), *size);
        }

        let igen = preset_data.child("igen").unwrap().data.clone();
        assert_eq!(read_u16(&igen, 0), 43); // the key range comes first
        assert_eq!(read_u16(&igen, 2), 0 | (59 << 8));
        assert_eq!(read_u16(&igen, 4), 53); // the sample comes last
        assert_eq!(read_u16(&igen, 8), 43);
        assert_eq!(read_u16(&igen, 12), 52); // the fine tune
        assert_eq!(read_u16(&igen, 14) as i16, -25);
        assert_eq!(read_u16(&igen, 20), 53);
        assert_eq!(read_u16(&igen, 22), 1);

        let pgen = preset_data.child("pgen").unwrap().data.clone();
        assert_eq!(read_u16(&pgen, 0), 41); // the instrument

        let phdr = preset_data.child("phdr").unwrap().data.clone();
        assert_eq!(read_u16(&phdr, 38 + 20), 5); // the number of the second preset
        assert_eq!(read_u16(&phdr, 2 * 38 + 24), 2); // the bag index of the terminal record

        let shdr = preset_data.child("shdr").unwrap().data.clone();
        assert_eq!(read_u32(&shdr, 46 + 20), 49); // the start of "two"
        assert_eq!(read_u32(&shdr, 46 + 24), 49 + 12); // the end, the guard points included
        assert_eq!(read_u32(&shdr, 46 + 28), 49 + 1); // the loop start
        assert_eq!(read_u32(&shdr, 46 + 32), 49 + 4); // the loop end
        assert_eq!(read_u32(&shdr, 46 + 36), 44100);
        assert_eq!(shdr[46 + 40], 48);
        assert_eq!(shdr[46 + 41] as i8, -3);
        assert_eq!(read_u16(&shdr, 46 + 44), 1);
        assert_eq!(&shdr[2 * 46..2 * 46 + 4], b"EOS\0");
    }

    #[test]
    fn a_font_that_does_not_hold_together_is_an_error() {
        let mut font = small_font();
        font.instruments[0].zones[0].sample = 7;
        assert!(write_sf2(&font).is_err());

        let mut font = small_font();
        font.samples[1].loop_info = Some(SampleLoop { start: 2, end: 9 });
        assert!(write_sf2(&font).is_err());

        let mut font = small_font();
        font.presets[0].instrument = 3;
        assert!(write_sf2(&font).is_err());

        let mut font = small_font();
        font.instruments[0].zones[0].generators.push(Sf2GeneratorValue {
            generator: Sf2Generator::KeyRange,
            amount: 0,
        });
        assert!(write_sf2(&font).is_err());

        let mut font = small_font();
        font.instruments[0].zones[0].key_low = 70;
        assert!(write_sf2(&font).is_err());
    }
}
