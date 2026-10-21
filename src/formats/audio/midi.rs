use crate::error::Result;
use crate::formats::audio::seq::{
    read_seq, split_seq_bank, Seq, SeqEvent, SeqEventType, END_META, META_STATUS, TEMPO_META,
};

pub const MIDI_LOOP_START_MARKER: &str = "loopStart";
pub const MIDI_LOOP_END_MARKER: &str = "loopEnd";
const MARKER_META: u8 = 0x06;

pub type BendRanges = [Option<u8>; 128];

/// The bend ranges of a sound bank without instruments: no program bends.
pub fn no_bend_ranges() -> BendRanges {
    [None; 128]
}

pub fn pitch_bend_ranges(vab: &crate::formats::vab2::Vab) -> BendRanges {
    let mut ranges: BendRanges = [None; 128];
    for program in 0..128 {
        if let Some(tones) = vab.tones_of(program) {
            if !tones.is_empty() {
                ranges[program] = Some(tones[0].pbmax);
            }
        }
    }
    ranges
}

struct TrackWriter {
    bytes: Vec<u8>,
    last_tick: u32,
}

impl TrackWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            last_tick: 0,
        }
    }

    fn delta(&mut self, tick: u32) {
        let mut value = tick.wrapping_sub(self.last_tick);
        let mut groups = [0u8; 5];
        let mut count = 0;
        loop {
            groups[count] = (value & 0x7F) as u8;
            count += 1;
            value >>= 7;
            if value == 0 {
                break;
            }
        }
        while count > 1 {
            self.u8(groups[count - 1] | 0x80);
            count -= 1;
        }
        self.u8(groups[0]);
        self.last_tick = tick;
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn event(&mut self, tick: u32, bytes: &[u8]) {
        self.delta(tick);
        for &b in bytes {
            self.u8(b);
        }
    }

    fn meta(&mut self, tick: u32, meta_type: u8, data: &[u8]) {
        self.delta(tick);
        self.u8(META_STATUS);
        self.u8(meta_type);
        self.quantity(data.len() as u32);
        self.bytes.extend_from_slice(data);
    }

    fn text(&mut self, tick: u32, meta_type: u8, text: &str) {
        let data = text.as_bytes();
        self.meta(tick, meta_type, data);
    }

    fn quantity(&mut self, mut value: u32) {
        let mut groups = [0u8; 5];
        let mut count = 0;
        loop {
            groups[count] = (value & 0x7F) as u8;
            count += 1;
            value >>= 7;
            if value == 0 {
                break;
            }
        }
        while count > 1 {
            self.u8(groups[count - 1] | 0x80);
            count -= 1;
        }
        self.u8(groups[0]);
    }

    fn take(self) -> Vec<u8> {
        self.bytes
    }
}

fn tempo_bytes(tempo: u32) -> [u8; 3] {
    [
        (tempo >> 16) as u8,
        (tempo >> 8) as u8,
        tempo as u8,
    ]
}

fn write_bend_range(track: &mut TrackWriter, tick: u32, channel: u8, semitones: u8) {
    let control = 0xB0 | channel;
    track.event(tick, &[control, 101, 0]); // RPN MSB
    track.event(tick, &[control, 100, 0]); // RPN LSB
    track.event(tick, &[control, 6, semitones]); // Data Entry MSB
    track.event(tick, &[control, 38, 0]); // Data Entry LSB
    track.event(tick, &[control, 101, 127]); // RPN Null MSB
    track.event(tick, &[control, 100, 127]); // RPN Null LSB
}

fn write_event(track: &mut TrackWriter, event: &SeqEvent, bend_ranges: &BendRanges) {
    let channel_status = |kind: u8| kind | event.channel;
    match event.event_type {
        SeqEventType::NoteOff => {
            track.event(event.tick, &[channel_status(0x80), event.data1, event.data2]);
        }
        SeqEventType::NoteOn => {
            track.event(event.tick, &[channel_status(0x90), event.data1, event.data2]);
        }
        SeqEventType::KeyPressure => {
            track.event(event.tick, &[channel_status(0xA0), event.data1, event.data2]);
        }
        SeqEventType::Control => {
            track.event(event.tick, &[channel_status(0xB0), event.data1, event.data2]);
        }
        SeqEventType::Program => {
            track.event(event.tick, &[channel_status(0xC0), event.data1]);
            if let Some(range) = bend_ranges.get(event.data1 as usize).copied().flatten() {
                write_bend_range(track, event.tick, event.channel, range);
            }
        }
        SeqEventType::ChannelPressure => {
            track.event(event.tick, &[channel_status(0xD0), event.data1]);
        }
        SeqEventType::PitchBend => {
            track.event(event.tick, &[channel_status(0xE0), event.data1, event.data2]);
        }
        SeqEventType::Tempo => {
            let tempo_bytes = [
                (event.tempo >> 16) as u8,
                (event.tempo >> 8) as u8,
                event.tempo as u8,
            ];
            track.meta(event.tick, TEMPO_META, &tempo_bytes);
        }
        SeqEventType::LoopStart => {
            track.text(event.tick, MARKER_META, MIDI_LOOP_START_MARKER);
        }
        SeqEventType::LoopEnd => {
            track.text(event.tick, MARKER_META, MIDI_LOOP_END_MARKER);
        }
        SeqEventType::End => {
            track.meta(event.tick, END_META, &[]);
        }
    }
}

pub fn convert_seq_to_midi(seq: &Seq, bend_ranges: &BendRanges) -> Result<Vec<u8>> {
    let mut track = TrackWriter::new();
    
    // Write tempo meta event at tick 0
    let tempo_bytes = [
        (seq.header.tempo >> 16) as u8,
        (seq.header.tempo >> 8) as u8,
        seq.header.tempo as u8,
    ];
    track.meta(0, TEMPO_META, &tempo_bytes);

    for event in &seq.events {
        write_event(&mut track, event, bend_ranges);
    }

    let track_bytes = track.take();
    
    // Build MIDI file (Format 0)
    let mut file = Vec::new();
    
    // MThd chunk
    file.extend_from_slice(b"MThd");
    file.extend_from_slice(&6u32.to_be_bytes()); // chunk size = 6
    file.extend_from_slice(&0u16.to_be_bytes()); // format 0
    file.extend_from_slice(&1u16.to_be_bytes()); // 1 track
    file.extend_from_slice(&seq.header.resolution.to_be_bytes()); // division
    
    // MTrk chunk
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&(track_bytes.len() as u32).to_be_bytes());
    file.extend_from_slice(&track_bytes);
    
    Ok(file)
}

/// The MIDI file of every sequence of a bank: the tracks of a music font, in order.
pub fn convert_seq_bank_to_midi(bank: &[u8], bend_ranges: &BendRanges) -> Result<Vec<Vec<u8>>> {
    let mut files = Vec::new();
    for track in split_seq_bank(bank)? {
        let seq = read_seq(track)?;
        files.push(convert_seq_to_midi(&seq, bend_ranges)?);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::vab2::{ProgramAttributes, ToneBlock, Vab, VabHeader};

    /// The (status, data...) bytes and the absolute tick of an event of the single track of a format 0 file.
    struct MidiEvent {
        tick: u32,
        bytes: Vec<u8>,
    }

    /// Reads back the track of a format 0 file, the way the reference tests read it.
    fn read_midi_track(file: &[u8]) -> (u16, Vec<MidiEvent>) {
        assert!(file.len() >= 22, "the file is shorter than its header");
        assert_eq!(&file[0..4], b"MThd");
        assert_eq!(u32::from_be_bytes(file[4..8].try_into().unwrap()), 6);
        assert_eq!(u16::from_be_bytes(file[8..10].try_into().unwrap()), 0);
        assert_eq!(u16::from_be_bytes(file[10..12].try_into().unwrap()), 1);
        let division = u16::from_be_bytes(file[12..14].try_into().unwrap());
        assert_eq!(&file[14..18], b"MTrk");
        assert_eq!(
            u32::from_be_bytes(file[18..22].try_into().unwrap()) as usize,
            file.len() - 22
        );

        let mut events = Vec::new();
        let mut at = 22usize;
        let mut tick = 0u32;
        while at < file.len() {
            let mut delta = 0u32;
            let mut byte;
            loop {
                byte = file[at];
                at += 1;
                delta = (delta << 7) | (byte & 0x7F) as u32;
                if byte & 0x80 == 0 {
                    break;
                }
            }
            tick += delta;
            let mut bytes = vec![file[at]];
            at += 1;
            let status = bytes[0];
            assert_ne!(status & 0x80, 0, "the writer never uses running status");
            let length = if status == 0xFF {
                bytes.push(file[at]);
                at += 1;
                let length = file[at] as usize;
                bytes.push(file[at]);
                at += 1;
                length
            } else if matches!(status & 0xF0, 0xC0 | 0xD0) {
                1
            } else {
                2
            };
            for _ in 0..length {
                bytes.push(file[at]);
                at += 1;
            }
            events.push(MidiEvent { tick, bytes });
        }
        assert_eq!(at, file.len());
        (division, events)
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

    /// A sequence with a program change, a loop, notes, a tempo and a pitch bend.
    fn the_sequence_of_the_reference() -> Seq {
        let mut seq = Seq::default();
        seq.header.resolution = 480;
        seq.header.tempo = 500000;
        seq.events = vec![
            event(0, SeqEventType::Program, 2, 4, 0),
            event(0, SeqEventType::LoopStart, 0, 127, 0),
            event(0, SeqEventType::NoteOn, 2, 60, 100),
            event(200, SeqEventType::NoteOn, 2, 60, 0),
            SeqEvent {
                tick: 300,
                event_type: SeqEventType::Tempo,
                channel: 0,
                data1: 0,
                data2: 0,
                tempo: 0x123456,
            },
            event(20000, SeqEventType::PitchBend, 2, 1, 2),
            event(20000, SeqEventType::LoopEnd, 0, 0, 0),
            event(20000, SeqEventType::End, 0, 0, 0),
        ];
        seq
    }

    #[test]
    fn a_midi_file_keeps_the_ticks_the_tempo_the_bend_range_and_the_loop_markers() {
        let seq = the_sequence_of_the_reference();
        let mut ranges: BendRanges = [None; 128];
        ranges[4] = Some(12);

        let file = convert_seq_to_midi(&seq, &ranges).unwrap();
        assert_eq!(convert_seq_to_midi(&seq, &ranges).unwrap(), file);

        let (division, events) = read_midi_track(&file);
        assert_eq!(division, 480);
        assert_eq!(events.len(), 15);
        assert_eq!(events[0].bytes, vec![0xFF, 0x51, 3, 0x07, 0xA1, 0x20]);
        assert_eq!(events[1].bytes, vec![0xC2, 4]);
        assert_eq!(events[2].bytes, vec![0xB2, 101, 0]);
        assert_eq!(events[4].bytes, vec![0xB2, 6, 12]);
        assert_eq!(
            events[8].bytes,
            vec![0xFF, 0x06, 9, b'l', b'o', b'o', b'p', b'S', b't', b'a', b'r', b't']
        );
        assert_eq!(events[10].tick, 200);
        assert_eq!(events[10].bytes, vec![0x92, 60, 0]);
        assert_eq!(events[11].tick, 300);
        assert_eq!(events[11].bytes, vec![0xFF, 0x51, 3, 0x12, 0x34, 0x56]);
        assert_eq!(events[12].tick, 20000);
        assert_eq!(events[12].bytes, vec![0xE2, 1, 2]);
        assert_eq!(
            events[13].bytes,
            vec![0xFF, 0x06, 7, b'l', b'o', b'o', b'p', b'E', b'n', b'd']
        );
        assert_eq!(events[14].bytes, vec![0xFF, 0x2F, 0]);
    }

    #[test]
    fn a_program_without_a_bend_range_gets_no_rpn() {
        let seq = the_sequence_of_the_reference();
        let (_division, events) = read_midi_track(&convert_seq_to_midi(&seq, &[None; 128]).unwrap());
        assert_eq!(events.len(), 9);
    }

    #[test]
    fn the_bend_range_of_a_program_is_the_one_of_its_first_tone() {
        let mut programs = [ProgramAttributes::default(); 128];
        programs[2].tones = 2;
        let mut block = ToneBlock {
            program: 2,
            ..ToneBlock::default()
        };
        block.tones[0].pbmax = 12;
        block.tones[1].pbmax = 2;

        let vab = Vab {
            header: VabHeader::default(),
            programs,
            tone_blocks: vec![block],
            vag_sizes: [0; 256],
        };

        let ranges = pitch_bend_ranges(&vab);
        assert_eq!(ranges[2], Some(12));
        assert_eq!(ranges[0], None);
    }

    #[test]
    fn a_program_of_a_sequence_that_the_bank_does_not_hold_does_not_stop_the_conversion() {
        let mut seq = Seq::default();
        seq.header.resolution = 480;
        seq.header.tempo = 500000;
        seq.events = vec![event(0, SeqEventType::Program, 0, 200, 0)];

        let file = convert_seq_to_midi(&seq, &[None; 128]).unwrap();
        let (_division, events) = read_midi_track(&file);
        assert_eq!(events[1].bytes, vec![0xC0, 200]);
    }

    #[test]
    fn the_tempo_of_a_sequence_is_written_as_three_bytes() {
        assert_eq!(tempo_bytes(0x123456), [0x12, 0x34, 0x56]);
    }

    #[test]
    fn the_delta_times_of_a_track_are_variable_length_quantities() {
        let mut track = TrackWriter::new();
        track.delta(100);
        track.delta(200);
        assert_eq!(track.bytes, vec![0x64, 0x64]);
    }
}
