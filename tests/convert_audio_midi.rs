//! The command that writes the MIDI files of a music font, from the end to end.

use dw_extract::cli::{run, Cli, Commands};
use std::path::{Path, PathBuf};

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
fn vab(program: u16, tone_program: i16, tone_vag: i16, ps: u16, vs: u16, vb_size: usize) -> Bytes {
    let mut b = Bytes::new();
    let vh_size = 32 + 128 * 16 + 1 * 16 * 32 + 256 * 2;
    b.text("pBAV")
        .le32(7) // version
        .le32(0) // id
        .le32((vh_size + vb_size) as u32) // fsize
        .le16(0) // reserved0
        .le16(ps)
        .le16(1) // ts
        .le16(vs)
        .u8(0) // mvol
        .u8(0) // pan
        .u8(0) // attr1
        .u8(0) // attr2
        .le32(0); // reserved1

    for index in 0..128 {
        if index == program as usize {
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
        .le16(tone_program as u16)
        .le16(tone_vag as u16)
        .le16(0)
        .le16(0)
        .le16(0)
        .le16(0);
    b.fill(15 * 32, 0);

    for index in 0..256 {
        b.le16(if index == 1 { 2 } else { 0 });
    }

    b
}

/// A sequence with a program change, a note and its end.
fn seq() -> Bytes {
    let mut b = Bytes::new();
    b.text("pQES")
        .be32(1) // version
        .be16(480) // resolution
        .be24(500000) // tempo
        .u8(1) // rhythm numerator
        .u8(4) // rhythm denominator
        .u8(0)
        .u8(0xC1)
        .u8(5) // program 5 on channel 1
        .u8(0)
        .u8(0x91)
        .u8(60)
        .u8(100) // note on
        .u8(0)
        .u8(0xFF)
        .u8(0x2F)
        .u8(0); // the end
    b
}

/// The bank of the sequences: the table of the offsets and the sequences themselves. The first entry of the table is
/// its own size, which is also where the first sequence starts.
fn seq_bank(seqs: &[Bytes]) -> Bytes {
    let mut b = Bytes::new();
    let count = seqs.len();
    let table = count * 4;
    b.le32(table as u32);
    let mut at = table;
    for (index, seq) in seqs.iter().enumerate() {
        if index + 1 < count {
            at += seq.bytes().len();
            b.le32(at as u32);
        }
    }
    for seq in seqs {
        b.0.extend_from_slice(seq.bytes());
    }
    b
}

/// A music font: the four offsets of the header, the VH, the VB and the bank of the sequences.
fn music_font(vab: &Bytes, vb: &[u8], seq_bank: &Bytes) -> Bytes {
    let mut b = Bytes::new();
    let vh_offset = 16usize;
    let vb_offset = vh_offset + vab.bytes().len();
    let vb_end = vb_offset + vb.len();
    let seq_end = vb_end + seq_bank.bytes().len();
    b.le32(vh_offset as u32)
        .le32(vb_offset as u32)
        .le32(vb_end as u32)
        .le32(seq_end as u32);
    b.0.extend_from_slice(vab.bytes());
    b.0.extend_from_slice(vb);
    b.0.extend_from_slice(seq_bank.bytes());
    b
}

/// A directory of its own for the test, under the temporary directory of the system.
fn temp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("dw-extract-test-{}", name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn convert(input: &Path, output: &Path) {
    let cli = Cli {
        command: Commands::ConvertAudio {
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            format: "midi".into(),
            sample_rate: 37800,
            extract_vags: false,
            recursive: false,
        },
    };
    run(cli).unwrap();
}

/// The name of a MIDI file, with the track it holds.
fn midi_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().to_string()
}

#[test]
fn a_music_font_gives_one_midi_file_per_track() {
    let dir = temp_dir("midi-font");
    let font = music_font(&vab(5, 5, 1, 1, 1, 16), &[0xAA; 16], &seq_bank(&[seq()]));
    let input = dir.join("font.vhb");
    std::fs::write(&input, font.bytes()).unwrap();

    let output = dir.join("out");
    convert(&input, &output);

    let tracks: Vec<String> = std::fs::read_dir(&output)
        .unwrap()
        .map(|entry| midi_name(&entry.unwrap().path()))
        .collect();
    assert_eq!(tracks, vec!["track_00.mid"]);

    let midi = std::fs::read(output.join("track_00.mid")).unwrap();
    assert_eq!(&midi[0..4], b"MThd");
    assert_eq!(u16::from_be_bytes([midi[8], midi[9]]), 0); // format 0
    assert_eq!(u16::from_be_bytes([midi[12], midi[13]]), 480); // the resolution of the sequence
    assert_eq!(&midi[14..18], b"MTrk");
}

#[test]
fn a_plain_sequence_gives_a_midi_file_with_the_name_of_the_file() {
    let dir = temp_dir("midi-seq");
    let input = dir.join("song.seq");
    std::fs::write(&input, seq().bytes()).unwrap();

    let output = dir.join("out");
    convert(&input, &output);

    let files: Vec<String> = std::fs::read_dir(&output)
        .unwrap()
        .map(|entry| midi_name(&entry.unwrap().path()))
        .collect();
    assert_eq!(files, vec!["song.mid"]);

    let midi = std::fs::read(output.join("song.mid")).unwrap();
    assert_eq!(&midi[0..4], b"MThd");
}

#[test]
fn two_music_fonts_in_a_directory_get_a_directory_each() {
    let dir = temp_dir("midi-many");
    let font = music_font(&vab(5, 5, 1, 1, 1, 16), &[0xAA; 16], &seq_bank(&[seq()]));
    std::fs::write(dir.join("one.vhb"), font.bytes()).unwrap();
    std::fs::write(dir.join("two.vhb"), font.bytes()).unwrap();

    let output = dir.join("out");
    convert(&dir, &output);

    assert!(output.join("one").join("track_00.mid").exists());
    assert!(output.join("two").join("track_00.mid").exists());
}

#[test]
fn a_file_that_is_not_a_sequence_is_an_error() {
    let dir = temp_dir("midi-bad");
    let input = dir.join("other.bin");
    std::fs::write(&input, b"not a sequence").unwrap();

    let output = dir.join("out");
    let cli = Cli {
        command: Commands::ConvertAudio {
            input: input.clone(),
            output: output.clone(),
            format: "midi".into(),
            sample_rate: 37800,
            extract_vags: false,
            recursive: false,
        },
    };
    assert!(run(cli).is_err());
}
