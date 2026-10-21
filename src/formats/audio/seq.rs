use crate::error::{DwExtractError, Result};
use crate::formats::byte_reader::ByteReader;
use std::io::Read;

const SEQ_MAGIC: &[u8] = b"pQES";
/// The size of the header of a SEQ, the magic included.
const HEADER_SIZE: usize = 15;
const MAX_DELTA_BYTES: usize = 4;
pub const META_STATUS: u8 = 0xFF;
pub const TEMPO_META: u8 = 0x51;
pub const END_META: u8 = 0x2F;
pub const SYSTEM_STATUS: u8 = 0xF0;

pub const NRPN_MSB: u8 = 99;
pub const NRPN_LSB: u8 = 98;
pub const DATA_ENTRY_MSB: u8 = 6;
pub const DATA_ENTRY_LSB: u8 = 38;
pub const RPN_NULL: u8 = 127;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeqHeader {
    pub version: u32,
    pub resolution: u16,
    pub tempo: u32,
    pub rhythm_numerator: u8,
    pub rhythm_denominator: u8,
}

impl SeqHeader {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let version = u32::from_be_bytes(buf);
        reader.read_exact(&mut buf[..2])?;
        let resolution = u16::from_be_bytes([buf[0], buf[1]]);
        reader.read_exact(&mut buf[..3])?;
        let tempo = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        let rhythm_numerator = {
            let mut b = [0u8; 1];
            reader.read_exact(&mut b)?;
            b[0]
        };
        let rhythm_denominator = {
            let mut b = [0u8; 1];
            reader.read_exact(&mut b)?;
            b[0]
        };
        Ok(Self {
            version,
            resolution,
            tempo,
            rhythm_numerator,
            rhythm_denominator,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SeqEventType {
    #[default]
    NoteOff = 0,
    NoteOn,
    KeyPressure,
    Control,
    Program,
    ChannelPressure,
    PitchBend,
    Tempo,
    LoopStart,
    LoopEnd,
    End,
}

impl SeqEventType {
    pub fn from_status(status: u8) -> Self {
        match status & 0xF0 {
            0x80 => SeqEventType::NoteOff,
            0x90 => SeqEventType::NoteOn,
            0xA0 => SeqEventType::KeyPressure,
            0xB0 => SeqEventType::Control,
            0xC0 => SeqEventType::Program,
            0xD0 => SeqEventType::ChannelPressure,
            _ => SeqEventType::PitchBend,
        }
    }

    pub fn has_two_data_bytes(self) -> bool {
        !matches!(self, SeqEventType::Program | SeqEventType::ChannelPressure)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeqEvent {
    pub tick: u32,
    pub event_type: SeqEventType,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
    pub tempo: u32,
}

pub const LOOP_START: u8 = 20;
pub const LOOP_END: u8 = 30;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seq {
    pub header: SeqHeader,
    pub events: Vec<SeqEvent>,
    pub size: usize,
}

struct SeqCursor<'a> {
    reader: crate::formats::byte_reader::ByteReader<'a>,
    offset: usize,
}

impl<'a> SeqCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            reader: crate::formats::byte_reader::ByteReader::new(bytes),
            offset: HEADER_SIZE,
        }
    }

    fn offset(&self) -> usize {
        self.offset
    }

    fn peek(&self) -> Option<u8> {
        self.reader.u8(self.offset)
    }

    fn next(&mut self) -> Option<u8> {
        let value = self.reader.u8(self.offset);
        if value.is_some() {
            self.offset += 1;
        }
        value
    }

    fn data(&mut self) -> Option<u8> {
        let value = self.next();
        value.filter(|&v| v < 0x80)
    }

    fn delta(&mut self) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..MAX_DELTA_BYTES {
            let byte = self.next()?;
            value = (value << 7) | ((byte & 0x7F) as u32);
            if (byte & 0x80) == 0 {
                return Some(value);
            }
        }
        None
    }
}

fn channel_event_type(status: u8) -> SeqEventType {
    match status & 0xF0 {
        0x80 => SeqEventType::NoteOff,
        0x90 => SeqEventType::NoteOn,
        0xA0 => SeqEventType::KeyPressure,
        0xB0 => SeqEventType::Control,
        0xC0 => SeqEventType::Program,
        0xD0 => SeqEventType::ChannelPressure,
        _ => SeqEventType::PitchBend,
    }
}

fn has_two_data_bytes(event_type: SeqEventType) -> bool {
    !matches!(event_type, SeqEventType::Program | SeqEventType::ChannelPressure)
}

fn is_control(event: &SeqEvent, controller: u8) -> bool {
    matches!(event.event_type, SeqEventType::Control) && event.data1 == controller
}

fn fold_loop_markers(events: Vec<SeqEvent>) -> Vec<SeqEvent> {
    let mut folded = Vec::new();
    let mut i = 0;
    while i < events.len() {
        let event = &events[i];
        if is_control(event, 99) && event.data2 == 20
            && i + 1 < events.len()
            && is_control(&events[i + 1], 6)
            && events[i + 1].channel == event.channel
        {
            folded.push(SeqEvent {
                tick: event.tick,
                event_type: SeqEventType::LoopStart,
                channel: event.channel,
                data1: events[i + 1].data2,
                data2: 0,
                tempo: 0,
            });
            i += 2;
        } else if is_control(event, 99) && event.data2 == 30 {
            folded.push(SeqEvent {
                tick: event.tick,
                event_type: SeqEventType::LoopEnd,
                channel: event.channel,
                data1: 0,
                data2: 0,
                tempo: 0,
            });
            i += 1;
        } else {
            folded.push(event.clone());
            i += 1;
        }
    }
    folded
}

pub fn read_seq(bytes: &[u8]) -> Result<Seq> {
    if bytes.len() < SEQ_MAGIC.len() || &bytes[0..SEQ_MAGIC.len()] != SEQ_MAGIC {
        return Err(DwExtractError::InvalidFormat(
            "not a SEQ: it does not start with pQES".into(),
        ));
    }
    if bytes.len() < HEADER_SIZE {
        return Err(DwExtractError::InvalidFormat(format!(
            "SEQ of {} bytes is shorter than its header",
            bytes.len()
        )));
    }

    let mut cursor = SeqCursor::new(bytes);
    let mut seq = Seq::default();

    let reader = ByteReader::new(bytes);
    let byte_at = |offset: usize| reader.u8(offset).unwrap_or(0) as u32;

    seq.header.version = (byte_at(4) << 24) | (byte_at(5) << 16) | (byte_at(6) << 8) | byte_at(7);
    seq.header.resolution = ((byte_at(8) << 8) | byte_at(9)) as u16;
    seq.header.tempo = (byte_at(10) << 16) | (byte_at(11) << 8) | byte_at(12);
    seq.header.rhythm_numerator = byte_at(13) as u8;
    seq.header.rhythm_denominator = byte_at(14) as u8;

    if seq.header.resolution == 0 || seq.header.resolution >= 0x8000 {
        return Err(DwExtractError::InvalidFormat(format!(
            "SEQ resolution {} is not a number of ticks per quarter note", seq.header.resolution
        )));
    }

    let mut events = Vec::new();
    let mut tick = 0u64;
    let mut status = 0u8;

    loop {
        let event_offset = cursor.offset();
        let delta = cursor.delta().ok_or_else(|| {
            DwExtractError::InvalidFormat(format!(
                "SEQ event at 0x{:x}: the delta time is cut or longer than 4 bytes", event_offset
            ))
        })?;
        tick += delta as u64;
        if tick > u32::MAX as u64 {
            return Err(DwExtractError::InvalidFormat(format!(
                "SEQ event at 0x{:x} is past the last tick a SEQ can hold", event_offset
            )));
        }

        let first = cursor.peek().ok_or_else(|| {
            DwExtractError::InvalidFormat("SEQ ends without its end event".into())
        })?;

        if (first & 0x80) != 0 {
            status = first;
            cursor.next();
        } else if status == 0 {
            return Err(DwExtractError::InvalidFormat(format!(
                "SEQ event at 0x{:x} has no status and none runs", event_offset
            )));
        }

        let mut event = SeqEvent {
            tick: tick as u32,
            event_type: SeqEventType::End,
            channel: 0,
            data1: 0,
            data2: 0,
            tempo: 0,
        };

        if status == META_STATUS {
            status = 0;
            let meta = cursor.next().ok_or_else(|| {
                DwExtractError::InvalidFormat("SEQ ends inside a meta event".into())
            })?;
            if meta == TEMPO_META {
                let b0 = cursor.next().ok_or_else(|| {
                    DwExtractError::InvalidFormat("SEQ ends inside a tempo event".into())
                })?;
                let b1 = cursor.next().ok_or_else(|| {
                    DwExtractError::InvalidFormat("SEQ ends inside a tempo event".into())
                })?;
                let b2 = cursor.next().ok_or_else(|| {
                    DwExtractError::InvalidFormat("SEQ ends inside a tempo event".into())
                })?;
                event.event_type = SeqEventType::Tempo;
                event.tempo = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
                events.push(event);
                continue;
            }
            if meta == END_META && cursor.next() == Some(0) {
                events.push(event);
                break;
            }
            return Err(DwExtractError::InvalidFormat(format!(
                "SEQ event at 0x{:x} is a meta event the sound library does not play", event_offset
            )));
        }

        if status >= SYSTEM_STATUS {
            return Err(DwExtractError::InvalidFormat(format!(
                "SEQ event at 0x{:x} has status 0x{:02X}, which a SEQ cannot hold", event_offset, status
            )));
        }

        event.event_type = channel_event_type(status);
        event.channel = status & 0x0F;
        
        let data1 = cursor.data().ok_or_else(|| {
            DwExtractError::InvalidFormat(format!(
                "SEQ event at 0x{:x} is cut or has a data byte above 0x7F", event_offset
            ))
        })?;
        let data2 = if event.event_type.has_two_data_bytes() {
            cursor.data().ok_or_else(|| {
                DwExtractError::InvalidFormat(format!(
                    "SEQ event at 0x{:x} is cut or has a data byte above 0x7F", event_offset
                ))
            })?
        } else {
            0
        };
        
        event.data1 = data1;
        event.data2 = data2;
        events.push(event);
    }

    seq.events = fold_loop_markers(events);
    seq.size = cursor.offset();

    Ok(seq)
}

pub fn split_seq_bank(bank: &[u8]) -> Result<Vec<&[u8]>> {
    use crate::formats::byte_reader::ByteReader;
    let reader = ByteReader::new(bank);
    let first = reader.u32(0).ok_or_else(|| {
        DwExtractError::InvalidFormat("SEQ bank does not start with a table of offsets".into())
    })?;
    
    if first < 4 || first % 4 != 0 || first as usize > bank.len() {
        return Err(DwExtractError::InvalidFormat(format!(
            "SEQ bank of {} bytes does not start with a table of offsets", bank.len()
        )));
    }

    let count = first as usize / 4;
    let mut seqs = Vec::with_capacity(count);
    
    for i in 0..count {
        let start = reader.u32(i * 4).unwrap_or(0) as usize;
        let end = if i + 1 < count {
            reader.u32((i + 1) * 4).unwrap_or(0) as usize
        } else {
            bank.len()
        };
        
        if start < first as usize || end < start || end > bank.len() {
            return Err(DwExtractError::InvalidFormat(format!(
                "SEQ {} of the bank spans 0x{:x}-0x{:x}, outside the bank after its table", i, start, end
            )));
        }
        
        seqs.push(&bank[start..end]);
    }

    Ok(seqs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes of a stream, built the way the tests of the reference build them.
    struct Bytes(Vec<u8>);

    impl Bytes {
        fn new() -> Self {
            Bytes(Vec::new())
        }

        fn u8(&mut self, value: u8) -> &mut Self {
            self.0.push(value);
            self
        }

        fn be16(&mut self, value: u16) -> &mut Self {
            self.u8((value >> 8) as u8);
            self.u8(value as u8)
        }

        fn be24(&mut self, value: u32) -> &mut Self {
            self.u8((value >> 16) as u8);
            self.be16(value as u16)
        }

        fn be32(&mut self, value: u32) -> &mut Self {
            self.be16((value >> 16) as u16);
            self.be16(value as u16)
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

        fn size(&self) -> usize {
            self.0.len()
        }
    }

    /// The header of a SEQ: the magic, the version, the resolution, the tempo and the rhythm.
    fn seq_header(resolution: u16, tempo: u32) -> Bytes {
        let mut bytes = Bytes::new();
        bytes
            .text("pQES")
            .be32(1)
            .be16(resolution)
            .be24(tempo)
            .u8(1)
            .u8(4);
        bytes
    }

    fn event(tick: u32, event_type: SeqEventType, channel: u8, data1: u8, data2: u8) -> SeqEvent {
        SeqEvent {
            tick,
            event_type,
            channel,
            data1,
            data2,
            tempo: 0,
        }
    }

    #[test]
    fn the_header_the_running_status_the_tempo_without_a_length_byte_and_the_end() {
        let mut b = seq_header(480, 500000);
        b.u8(0).u8(0xC1).u8(5); // program 5 on channel 1
        b.u8(0).u8(0x91).u8(60).u8(100); // note on
        b.u8(0x83).u8(0x60).u8(60).u8(0); // delta 480, running status: note on with velocity 0
        b.u8(0).u8(0xFF).u8(0x51).be24(400000); // tempo, no length byte
        b.u8(0x10).u8(0xE1).u8(0).u8(0x40); // pitch bend centre
        b.u8(0).u8(0xFF).u8(0x2F).u8(0);
        b.u8(0xAA); // after the end: not read

        let seq = read_seq(b.bytes()).unwrap();
        assert_eq!(
            seq.header,
            SeqHeader {
                version: 1,
                resolution: 480,
                tempo: 500000,
                rhythm_numerator: 1,
                rhythm_denominator: 4
            }
        );
        assert_eq!(seq.events.len(), 6);
        assert_eq!(seq.events[0], event(0, SeqEventType::Program, 1, 5, 0));
        assert_eq!(seq.events[1], event(0, SeqEventType::NoteOn, 1, 60, 100));
        assert_eq!(seq.events[2], event(480, SeqEventType::NoteOn, 1, 60, 0));
        assert_eq!(seq.events[3].event_type, SeqEventType::Tempo);
        assert_eq!(seq.events[3].tempo, 400000);
        assert_eq!(seq.events[4], event(496, SeqEventType::PitchBend, 1, 0, 0x40));
        assert_eq!(seq.events[5].event_type, SeqEventType::End);
        assert_eq!(seq.size, b.size() - 1);
    }

    #[test]
    fn the_loop_markers_of_the_library_become_loop_events() {
        let mut b = seq_header(480, 500000);
        b.u8(0).u8(0xB0).u8(99).u8(20); // loop start
        b.u8(0).u8(6).u8(127); // running status: CC6 = count
        b.u8(0).u8(7).u8(100); // CC7 stays a control
        b.u8(0x60).u8(99).u8(30); // loop end
        b.u8(0).u8(0xB2).u8(99).u8(20); // CC99 = 20 without CC6 after it stays a control
        b.u8(0).u8(0xB2).u8(10).u8(64);
        b.u8(0).u8(0xFF).u8(0x2F).u8(0);

        let seq = read_seq(b.bytes()).unwrap();
        assert_eq!(seq.events.len(), 6);
        assert_eq!(seq.events[0], event(0, SeqEventType::LoopStart, 0, 127, 0));
        assert_eq!(seq.events[1], event(0, SeqEventType::Control, 0, 7, 100));
        assert_eq!(seq.events[2], event(0x60, SeqEventType::LoopEnd, 0, 0, 0));
        assert_eq!(seq.events[3], event(0x60, SeqEventType::Control, 2, 99, 20));
    }

    #[test]
    fn a_malformed_stream_is_an_error() {
        let with_events = |events: &[u8]| {
            let mut b = seq_header(480, 500000);
            for value in events {
                b.u8(*value);
            }
            read_seq(b.bytes())
        };

        assert!(with_events(&[0, 0x90, 60, 100, 0, 0xFF, 0x2F, 0]).is_ok());
        assert!(with_events(&[0, 0x90, 60, 100]).is_err()); // no end
        assert!(with_events(&[0, 60, 100]).is_err()); // running status with none
        assert!(with_events(&[0, 0xFF, 0x51, 1, 2, 3, 0, 60, 100]).is_err()); // a meta ends running status
        assert!(with_events(&[0, 0xFF, 0x01, 2, b'h', b'i']).is_err()); // other meta events
        assert!(with_events(&[0, 0xF0, 1, 0xF7]).is_err()); // system exclusive
        assert!(with_events(&[0, 0x90, 0x80, 100]).is_err()); // data byte above 0x7F
        assert!(with_events(&[0x80, 0x80, 0x80, 0x80, 0, 0xFF, 0x2F, 0]).is_err()); // 5-byte delta
        assert!(with_events(&[0, 0xFF, 0x51, 1]).is_err()); // cut tempo

        let mut cut = Bytes::new();
        cut.text("pQES").u8(0);
        assert!(read_seq(cut.bytes()).is_err());

        let mut other = Bytes::new();
        other.text("MThd").fill(20, 0);
        assert!(read_seq(other.bytes()).is_err());

        let mut no_resolution = seq_header(0, 500000);
        no_resolution.u8(0).u8(0xFF).u8(0x2F).u8(0);
        assert!(read_seq(no_resolution.bytes()).is_err());
    }

    #[test]
    fn a_bank_of_sequences_holds_the_offsets_from_its_start() {
        let mut b = Bytes::new();
        b.le32(12).le32(16).le32(20).fill(4, 1).fill(4, 2).fill(6, 3);

        let seqs = split_seq_bank(b.bytes()).unwrap();
        assert_eq!(seqs.len(), 3);
        assert_eq!(seqs[0].len(), 4);
        assert_eq!(seqs[2].len(), 6);
        assert_eq!(seqs[1][0], 2);

        let mut no_table = Bytes::new();
        no_table.le32(0);
        assert!(split_seq_bank(no_table.bytes()).is_err());

        let mut odd = Bytes::new();
        odd.le32(6).fill(4, 0);
        assert!(split_seq_bank(odd.bytes()).is_err());

        let mut backwards = Bytes::new();
        backwards.le32(8).le32(4);
        assert!(split_seq_bank(backwards.bytes()).is_err());

        let mut past = Bytes::new();
        past.le32(8).le32(20);
        assert!(split_seq_bank(past.bytes()).is_err());

        let mut outside = Bytes::new();
        outside.le32(16);
        assert!(split_seq_bank(outside.bytes()).is_err());
    }

    #[test]
    fn the_status_of_an_event_tells_its_kind() {
        assert_eq!(SeqEventType::from_status(0x80), SeqEventType::NoteOff);
        assert_eq!(SeqEventType::from_status(0x90), SeqEventType::NoteOn);
        assert_eq!(SeqEventType::from_status(0xB0), SeqEventType::Control);
        assert_eq!(SeqEventType::from_status(0xC0), SeqEventType::Program);
        assert_eq!(SeqEventType::from_status(0xE0), SeqEventType::PitchBend);
    }
}
