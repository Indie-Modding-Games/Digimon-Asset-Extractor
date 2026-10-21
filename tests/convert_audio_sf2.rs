//! The command that writes the SoundFont of a sound bank, from the end to end.

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

/// A VAB with one program that has one tone, and a size table of one sample.
fn vab(vb_size: usize) -> Bytes {
    let mut b = Bytes::new();
    let vh_size = 32 + 128 * 16 + 1 * 16 * 32 + 256 * 2;
    b.text("pBAV")
        .le32(7) // version
        .le32(0) // id
        .le32((vh_size + vb_size) as u32) // fsize
        .le16(0) // reserved0
        .le16(1) // ps: one program
        .le16(1) // ts: one tone
        .le16(1) // vs: one sample
        .u8(127) // mvol
        .u8(64) // pan
        .u8(0) // attr1
        .u8(0) // attr2
        .le32(0); // reserved1

    for index in 0..128 {
        if index == 5 {
            b.u8(1) // tones
                .u8(127) // mvol
                .u8(0) // prior
                .u8(0) // mode
                .u8(64) // mpan
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
        .le16(0x80FF) // adsr1
        .le16(0x5FC0) // adsr2
        .le16(5) // the program of the tone
        .le16(1) // the sample of the tone
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

/// A sound bank: the three offsets of the header, the VH and the VB.
fn sound_bank(vab: &Bytes, vb: &[u8]) -> Bytes {
    let mut b = Bytes::new();
    let vh_offset = 12usize;
    let vb_offset = vh_offset + vab.bytes().len();
    let vb_end = vb_offset + vb.len();
    b.le32(vh_offset as u32)
        .le32(vb_offset as u32)
        .le32(vb_end as u32);
    b.0.extend_from_slice(vab.bytes());
    b.0.extend_from_slice(vb);
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
            format: "sf2".into(),
            sample_rate: 37800,
            extract_vags: false,
            recursive: false,
        },
    };
    run(cli).unwrap();
}

/// The position of a chunk of a RIFF file, if it holds one.
fn chunk(data: &[u8], id: &[u8; 4]) -> Option<usize> {
    data.windows(4).position(|window| window == id)
}

#[test]
fn a_sound_bank_gives_a_soundfont() {
    let dir = temp_dir("sf2-bank");
    let bank = sound_bank(&vab(16), &[0xAA; 16]);
    let input = dir.join("bank.vhb");
    std::fs::write(&input, bank.bytes()).unwrap();

    let output = dir.join("out");
    convert(&input, &output);

    let font = std::fs::read(output.join("output.sf2")).unwrap();
    assert_eq!(&font[0..4], b"RIFF");
    assert_eq!(&font[8..12], b"sfbk");
    assert_eq!(u32::from_le_bytes([font[4], font[5], font[6], font[7]]) as usize, font.len() - 8);
    // The tables of the font: the presets, the instruments and the samples.
    assert!(chunk(&font, b"phdr").is_some());
    assert!(chunk(&font, b"inst").is_some());
    assert!(chunk(&font, b"shdr").is_some());
    // The samples of the font, with their 16-bit samples.
    assert!(chunk(&font, b"sdta").is_some());
    assert!(chunk(&font, b"smpl").is_some());
}
