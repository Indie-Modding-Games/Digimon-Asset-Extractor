use crate::error::{DwExtractError, Result};
use crate::formats::video::xa_adpcm::{read_xa_coding, XaCoding, XaDecoder};
use std::collections::HashMap;
use std::io::{ErrorKind, Read};

/// STR stream files: mode 2 sectors read at a fixed rate, where video sectors (form 1, a 32-byte StHEADER and a chunk
/// of a frame's bitstream) are interleaved with XA audio sectors (form 2). A file must be read as raw sectors with
/// their subheaders; the 2048-byte view of an ISO image loses the audio and the headers' sector types.
pub const STR_SECTOR_SIZE: usize = 2352;
/// The sync pattern every sector starts with.
pub const STR_SYNC_SIZE: usize = 12;
/// The header of a mode 2 sector: the minute, the frame, the sector and the mode.
pub const STR_HEADER_SIZE: usize = 4;
/// Two copies of the file, the channel, the submode and the coding of a sector.
pub const STR_SUBHEADER_SIZE: usize = 8;
/// Where the user data of a sector starts.
pub const STR_USER_DATA_OFFSET: usize = STR_SYNC_SIZE + STR_HEADER_SIZE + STR_SUBHEADER_SIZE;
/// The user data of a form 1 sector: a video sector's header and chunk.
pub const STR_FORM1_DATA_SIZE: usize = 2048;
/// The user data of a form 2 sector: 18 sound groups and the bytes after them.
pub const STR_FORM2_DATA_SIZE: usize = 2324;
/// The size of the StHEADER that starts every video sector.
pub const STR_VIDEO_HEADER_SIZE: usize = 0x20;
/// The bytes of a frame's bitstream a video sector holds.
pub const STR_VIDEO_SECTOR_SIZE: usize = 0x7E0;
/// The user data of an audio sector.
pub const STR_AUDIO_SECTOR_SIZE: usize = STR_FORM2_DATA_SIZE;

const SYNC_PATTERN: [u8; STR_SYNC_SIZE] = [
    0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
];
const ST_MARKER: u16 = 0x0160;
const ST_VIDEO_TYPE: u16 = 0x8001;

/// Bits of a sector's submode.
pub mod submode {
    pub const END_OF_RECORD: u8 = 0x01;
    pub const VIDEO: u8 = 0x02;
    pub const AUDIO: u8 = 0x04;
    pub const DATA: u8 = 0x08;
    pub const FORM2: u8 = 0x20;
    pub const REAL_TIME: u8 = 0x40;
    pub const END_OF_FILE: u8 = 0x80;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrSectorType {
    Video,
    Audio,
    Padding,
    Unknown,
}

/// The subheader of a sector: the file and the channel of an interleaved stream the sector belongs to, what it holds
/// and, for audio, how the samples are coded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrSubHeader {
    pub file_number: u8,
    pub channel_number: u8,
    pub submode: u8,
    pub coding: u8,
}

impl StrSubHeader {
    /// Reads the subheader of a sector: two copies of the same four bytes.
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; STR_SUBHEADER_SIZE];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_bytes(&buf))
    }

    fn from_bytes(buf: &[u8; STR_SUBHEADER_SIZE]) -> Self {
        Self {
            file_number: buf[0],
            channel_number: buf[1],
            submode: buf[2],
            coding: buf[3],
        }
    }

    /// A video sector: it is not form 2 and it holds video or data.
    pub fn is_video(&self) -> bool {
        !self.is_form2() && (self.submode & (submode::VIDEO | submode::DATA)) != 0
    }

    /// An audio sector: it is form 2 and it holds audio.
    pub fn is_audio(&self) -> bool {
        self.is_form2() && (self.submode & submode::AUDIO) != 0
    }

    pub fn is_form2(&self) -> bool {
        (self.submode & submode::FORM2) != 0
    }

    pub fn is_eof(&self) -> bool {
        (self.submode & submode::END_OF_FILE) != 0
    }

    pub fn is_eor(&self) -> bool {
        (self.submode & submode::END_OF_RECORD) != 0
    }
}

/// A video sector: its StHEADER and its chunk of the frame.
#[derive(Debug, Clone)]
pub struct StrVideoSector {
    pub subheader: StrSubHeader,
    pub sector_in_frame: u16,
    pub sectors_in_frame: u16,
    pub frame_number: u32,
    pub frame_bytes: u32,
    pub width: u16,
    pub height: u16,
    pub headm: u32,
    pub headv: u32,
    pub chunk: Vec<u8>,
}

impl StrVideoSector {
    /// Reads the video header at the start of a sector's user data and its chunk of the frame.
    fn parse(subheader: StrSubHeader, user_data: &[u8]) -> Result<Self> {
        if user_data.len() < STR_VIDEO_HEADER_SIZE + STR_VIDEO_SECTOR_SIZE {
            return Err(DwExtractError::InvalidFormat(format!(
                "a video sector holds {} bytes of user data, too short for its header and chunk",
                user_data.len()
            )));
        }

        let marker = u16::from_le_bytes([user_data[0], user_data[1]]);
        let kind = u16::from_le_bytes([user_data[2], user_data[3]]);
        if marker != ST_MARKER || kind != ST_VIDEO_TYPE {
            return Err(DwExtractError::InvalidFormat(format!(
                "a data sector has {:#06x} {:#06x} where the video header goes",
                marker, kind
            )));
        }

        let sector_in_frame = u16::from_le_bytes([user_data[4], user_data[5]]);
        let sectors_in_frame = u16::from_le_bytes([user_data[6], user_data[7]]);
        let frame_number = u32::from_le_bytes([
            user_data[8],
            user_data[9],
            user_data[10],
            user_data[11],
        ]);
        let frame_bytes = u32::from_le_bytes([
            user_data[12],
            user_data[13],
            user_data[14],
            user_data[15],
        ]);
        let width = u16::from_le_bytes([user_data[16], user_data[17]]);
        let height = u16::from_le_bytes([user_data[18], user_data[19]]);
        let headm = u32::from_le_bytes([
            user_data[20],
            user_data[21],
            user_data[22],
            user_data[23],
        ]);
        let headv = u32::from_le_bytes([
            user_data[24],
            user_data[25],
            user_data[26],
            user_data[27],
        ]);
        let reserved = u32::from_le_bytes([
            user_data[28],
            user_data[29],
            user_data[30],
            user_data[31],
        ]);

        if reserved != 0 {
            return Err(DwExtractError::InvalidFormat(format!(
                "a video sector's reserved field is {:#x}",
                reserved
            )));
        }
        if sectors_in_frame == 0 || sector_in_frame >= sectors_in_frame {
            return Err(DwExtractError::InvalidFormat(format!(
                "a video sector says it is sector {} of {}",
                sector_in_frame, sectors_in_frame
            )));
        }

        let chunk = user_data[STR_VIDEO_HEADER_SIZE..STR_VIDEO_HEADER_SIZE + STR_VIDEO_SECTOR_SIZE].to_vec();

        Ok(Self {
            subheader,
            sector_in_frame,
            sectors_in_frame,
            frame_number,
            frame_bytes,
            width,
            height,
            headm,
            headv,
            chunk,
        })
    }
}

/// An XA audio sector.
#[derive(Debug, Clone)]
pub struct StrAudioSector {
    pub subheader: StrSubHeader,
    pub coding: XaCoding,
    pub eof: bool,
    /// The 18 sound groups of the sector and the bytes after them.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum StrSector {
    Video(StrVideoSector),
    Audio(StrAudioSector),
    Padding,
}

impl StrSector {
    /// What the sector holds.
    pub fn sector_type(&self) -> StrSectorType {
        match self {
            StrSector::Video(_) => StrSectorType::Video,
            StrSector::Audio(_) => StrSectorType::Audio,
            StrSector::Padding => StrSectorType::Padding,
        }
    }

    /// Reads the next sector of a stream: the sync pattern, the header, the subheader, the user data and the error
    /// correction bytes. Returns `None` at the end of the file.
    pub fn read<R: Read>(reader: &mut R) -> Result<Option<Self>> {
        let mut sync = [0u8; STR_SYNC_SIZE];
        match reader.read_exact(&mut sync) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        if sync != SYNC_PATTERN {
            return Err(DwExtractError::InvalidFormat(
                "a sector has no sync pattern; the file is not a raw sector stream".to_string(),
            ));
        }

        let mut header = [0u8; STR_HEADER_SIZE];
        reader.read_exact(&mut header)?;
        if header[3] != 0x02 {
            return Err(DwExtractError::InvalidFormat(format!(
                "unsupported sector mode {}",
                header[3]
            )));
        }

        let subheader = StrSubHeader::read(reader)?;
        let user_data_size = if subheader.is_form2() {
            STR_FORM2_DATA_SIZE
        } else {
            STR_FORM1_DATA_SIZE
        };

        let mut user_data = vec![0u8; user_data_size];
        reader.read_exact(&mut user_data)?;

        let mut error_correction = vec![0u8; STR_SECTOR_SIZE - STR_USER_DATA_OFFSET - user_data_size];
        reader.read_exact(&mut error_correction)?;

        if subheader.is_audio() {
            let coding = read_xa_coding(subheader.coding)?;
            return Ok(Some(StrSector::Audio(StrAudioSector {
                subheader,
                coding,
                eof: subheader.is_eof(),
                data: user_data,
            })));
        }

        if subheader.is_video() {
            let video = StrVideoSector::parse(subheader, &user_data)?;
            return Ok(Some(StrSector::Video(video)));
        }

        let has_type_bits = subheader.submode
            & (submode::VIDEO | submode::AUDIO | submode::DATA)
            != 0;
        if !has_type_bits && user_data.iter().all(|&byte| byte == 0) {
            return Ok(Some(StrSector::Padding));
        }

        Err(DwExtractError::InvalidFormat(format!(
            "a sector with submode {:#04x} is neither video, audio nor empty padding",
            subheader.submode
        )))
    }
}

#[derive(Debug, Clone)]
pub struct StrFrame {
    pub number: u32,
    pub width: u16,
    pub height: u16,
    pub bitstream: Vec<u8>,
}

/// Puts each frame together from its video sectors, which come in order, one frame after another.
#[derive(Debug, Default)]
pub struct StrFrameAssembler {
    first: Option<StrVideoSector>,
    data: Vec<u8>,
    next_sector: u16,
}

impl StrFrameAssembler {
    pub fn new() -> Self {
        Self {
            first: None,
            data: Vec::new(),
            next_sector: 0,
        }
    }

    /// Adds the next video sector. Returns the frame when this sector completes it, `None` otherwise. A frame whose
    /// sectors are missing is dropped when the next one starts.
    pub fn add_video_sector(&mut self, sector: StrVideoSector) -> Result<Option<StrFrame>> {
        if sector.sector_in_frame == self.next_sector {
            return self.take(sector);
        }

        self.reset();
        if sector.sector_in_frame != 0 {
            return Err(DwExtractError::InvalidFormat(format!(
                "sector {} of frame {} comes without the sectors before it",
                sector.sector_in_frame, sector.frame_number
            )));
        }
        self.take(sector)
    }

    fn take(&mut self, sector: StrVideoSector) -> Result<Option<StrFrame>> {
        if sector.sector_in_frame == 0 {
            if sector.frame_bytes as usize > sector.sectors_in_frame as usize * STR_VIDEO_SECTOR_SIZE {
                return Err(DwExtractError::InvalidFormat(format!(
                    "frame {} has {} bytes, more than its {} sectors hold",
                    sector.frame_number, sector.frame_bytes, sector.sectors_in_frame
                )));
            }
            self.first = Some(sector.clone());
            self.data.clear();
        } else {
            let first = self.first.as_ref().ok_or_else(|| {
                DwExtractError::InvalidFormat(format!(
                    "sector {} of frame {} comes without the sectors before it",
                    sector.sector_in_frame, sector.frame_number
                ))
            })?;
            if sector.frame_number != first.frame_number
                || sector.sectors_in_frame != first.sectors_in_frame
                || sector.frame_bytes != first.frame_bytes
                || sector.width != first.width
                || sector.height != first.height
                || sector.headm != first.headm
                || sector.headv != first.headv
            {
                return Err(DwExtractError::InvalidFormat(format!(
                    "sector {} of frame {} has a different header",
                    sector.sector_in_frame, sector.frame_number
                )));
            }
        }

        self.data.extend_from_slice(&sector.chunk);
        self.next_sector = sector.sector_in_frame + 1;
        if self.next_sector != sector.sectors_in_frame {
            return Ok(None);
        }

        let first = self.first.take().unwrap_or(sector);
        let mut data = std::mem::take(&mut self.data);
        self.reset();

        let frame_bytes = first.frame_bytes as usize;
        if data[frame_bytes..].iter().any(|&byte| byte != 0) {
            return Err(DwExtractError::InvalidFormat(format!(
                "frame {} has data in the padding after its {} bytes",
                first.frame_number, first.frame_bytes
            )));
        }
        data.truncate(frame_bytes);

        if data.len() < 8 {
            return Err(DwExtractError::InvalidFormat(format!(
                "frame {}: its bitstream is too short to hold headm and headv",
                first.frame_number
            )));
        }
        let headm = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let headv = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if headm != first.headm || headv != first.headv {
            return Err(DwExtractError::InvalidFormat(format!(
                "frame {}: headm and headv do not match its bitstream",
                first.frame_number
            )));
        }

        Ok(Some(StrFrame {
            number: first.frame_number,
            width: first.width,
            height: first.height,
            bitstream: data,
        }))
    }

    /// The number of the frame in progress, if a frame is missing sectors.
    pub fn pending_frame(&self) -> Option<u32> {
        self.first.as_ref().map(|sector| sector.frame_number)
    }

    fn reset(&mut self) {
        self.first = None;
        self.data.clear();
        self.next_sector = 0;
    }
}

/// An audio stream: the samples of one file and channel, decoded in the order its sectors came in.
#[derive(Debug, Clone)]
pub struct StrAudioTrack {
    pub file_number: u8,
    pub channel: u8,
    pub coding: XaCoding,
    /// The samples of every channel, one after the other.
    pub samples: Vec<i16>,
    /// How many sound units had a parameter byte that differs from its copy.
    pub parameter_mismatches: usize,
}

impl StrAudioTrack {
    /// A name for the coding of the track, to tell its files apart.
    pub fn coding_tag(&self) -> String {
        format!(
            "{}_{}hz_{}bit",
            if self.coding.channels == 2 {
                "stereo"
            } else {
                "mono"
            },
            self.coding.sample_rate,
            self.coding.bits_per_sample
        )
    }
}

pub struct StrFile {
    pub sectors: Vec<StrSector>,
    pub frames: Vec<StrFrame>,
    pub audio_tracks: Vec<StrAudioTrack>,
}

impl StrFile {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut sectors = Vec::new();

        while let Some(sector) = StrSector::read(reader)? {
            sectors.push(sector);
        }

        let mut assembler = StrFrameAssembler::new();
        let mut frames = Vec::new();
        let mut tracks: HashMap<(u8, u8), (XaDecoder, StrAudioTrack)> = HashMap::new();

        for sector in &sectors {
            match sector {
                StrSector::Video(video) => {
                    if let Some(frame) = assembler.add_video_sector(video.clone())? {
                        frames.push(frame);
                    }
                }
                StrSector::Audio(audio) => {
                    let key = (audio.subheader.file_number, audio.subheader.channel_number);
                    let entry = tracks.entry(key).or_insert_with(|| {
                        (
                            XaDecoder::new(),
                            StrAudioTrack {
                                file_number: audio.subheader.file_number,
                                channel: audio.subheader.channel_number,
                                coding: audio.coding,
                                samples: Vec::new(),
                                parameter_mismatches: 0,
                            },
                        )
                    });
                    let (decoder, track) = entry;
                    if track.coding != audio.coding {
                        return Err(DwExtractError::InvalidFormat(format!(
                            "audio sector of file {} channel {} changes its coding",
                            track.file_number, track.channel
                        )));
                    }
                    let mismatches = decoder.decode_sector(&audio.data, &audio.coding, &mut track.samples)?;
                    track.parameter_mismatches += mismatches;
                }
                StrSector::Padding => {}
            }
        }

        let audio_tracks: Vec<StrAudioTrack> = tracks.into_values().map(|(_, track)| track).collect();

        Ok(Self {
            sectors,
            frames,
            audio_tracks,
        })
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = std::io::Cursor::new(data);
        Self::read(&mut cursor)
    }

    pub fn extract_video(&self, output_dir: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;

        for frame in &self.frames {
            let frame_path = output_dir.join(format!("frame_{:05}.mdec", frame.number));
            std::fs::write(&frame_path, &frame.bitstream)?;
        }

        println!(
            "Extracted {} MDEC frames to {}",
            self.frames.len(),
            output_dir.display()
        );
        Ok(())
    }

    pub fn extract_audio(&self, output_dir: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;

        for track in &self.audio_tracks {
            let wav_path = output_dir.join(format!(
                "audio_f{}_ch{}_{}.wav",
                track.file_number,
                track.channel,
                track.coding_tag()
            ));

            crate::formats::wav::write_wav(
                &wav_path,
                &track.samples,
                track.coding.sample_rate,
                track.coding.channels as u16,
            )?;
            println!("Extracted audio track: {}", wav_path.display());
        }

        Ok(())
    }
}

pub fn parse_str_file(data: &[u8]) -> Result<StrFile> {
    StrFile::from_bytes(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::video::xa_adpcm::XA_SOUND_GROUPS;
    use crate::formats::video::xa_adpcm::XA_SOUND_GROUP_SIZE;

    const SYNC: [u8; STR_SYNC_SIZE] = [
        0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
    ];

    /// A raw sector: the sync, the header, the subheader, the user data and the error correction bytes.
    fn sector(submode: u8, coding: u8, user_data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(STR_SECTOR_SIZE);
        bytes.extend_from_slice(&SYNC);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x02]);
        bytes.extend_from_slice(&[0x01, 0x01, submode, coding]);
        bytes.extend_from_slice(&[0x01, 0x01, submode, coding]);
        bytes.extend_from_slice(user_data);
        bytes.resize(STR_SECTOR_SIZE, 0);
        bytes
    }

    /// A video sector's user data: the StHEADER and a chunk of the frame.
    fn video_user_data(
        sector_in_frame: u16,
        sectors_in_frame: u16,
        frame_bytes: u32,
        head: [u8; 8],
        chunk: &[u8],
    ) -> Vec<u8> {
        let mut data = vec![0u8; STR_FORM1_DATA_SIZE];
        data[0..2].copy_from_slice(&ST_MARKER.to_le_bytes());
        data[2..4].copy_from_slice(&ST_VIDEO_TYPE.to_le_bytes());
        data[4..6].copy_from_slice(&sector_in_frame.to_le_bytes());
        data[6..8].copy_from_slice(&sectors_in_frame.to_le_bytes());
        data[8..12].copy_from_slice(&1u32.to_le_bytes());
        data[12..16].copy_from_slice(&frame_bytes.to_le_bytes());
        data[16..18].copy_from_slice(&320u16.to_le_bytes());
        data[18..20].copy_from_slice(&240u16.to_le_bytes());
        data[20..24]
            .copy_from_slice(&u32::from_le_bytes([head[0], head[1], head[2], head[3]]).to_le_bytes());
        data[24..28]
            .copy_from_slice(&u32::from_le_bytes([head[4], head[5], head[6], head[7]]).to_le_bytes());
        data[STR_VIDEO_HEADER_SIZE..STR_VIDEO_HEADER_SIZE + chunk.len()].copy_from_slice(chunk);
        data
    }

    /// An audio sector's user data: 18 silent sound groups.
    fn audio_user_data() -> Vec<u8> {
        let mut data = vec![0u8; STR_FORM2_DATA_SIZE];
        for group in 0..XA_SOUND_GROUPS {
            for i in 0..16 {
                data[(group * XA_SOUND_GROUP_SIZE) + i] = 0x0C;
            }
        }
        data
    }

    #[test]
    fn the_sector_constants_follow_the_raw_cd_layout() {
        assert_eq!(STR_SECTOR_SIZE, 2352);
        assert_eq!(STR_USER_DATA_OFFSET, 24);
        assert_eq!(STR_FORM1_DATA_SIZE, 2048);
        assert_eq!(STR_FORM2_DATA_SIZE, 2324);
        assert_eq!(STR_VIDEO_SECTOR_SIZE, 2016);
    }

    #[test]
    fn a_video_sector_is_read_with_its_header_and_chunk() {
        let mut bytes = sector(
            submode::DATA | submode::REAL_TIME,
            0x00,
            &video_user_data(0, 1, 16, [7u8; 8], &[7u8; 16]),
        );
        bytes.extend_from_slice(&[0u8; STR_SECTOR_SIZE]);
        let mut reader = std::io::Cursor::new(bytes);

        let sector = StrSector::read(&mut reader).unwrap().unwrap();
        match sector {
            StrSector::Video(video) => {
                assert_eq!(video.subheader.file_number, 1);
                assert_eq!(video.subheader.channel_number, 1);
                assert_eq!(video.sector_in_frame, 0);
                assert_eq!(video.sectors_in_frame, 1);
                assert_eq!(video.frame_number, 1);
                assert_eq!(video.frame_bytes, 16);
                assert_eq!(video.width, 320);
                assert_eq!(video.height, 240);
                assert_eq!(video.chunk.len(), STR_VIDEO_SECTOR_SIZE);
                assert_eq!(video.chunk[0..16], [7u8; 16]);
            }
            other => panic!("expected a video sector, got {:?}", other),
        }
    }

    #[test]
    fn an_audio_sector_is_read_with_its_coding() {
        let bytes = sector(
            submode::AUDIO | submode::FORM2 | submode::REAL_TIME,
            0x01,
            &audio_user_data(),
        );
        let mut reader = std::io::Cursor::new(bytes);

        let sector = StrSector::read(&mut reader).unwrap().unwrap();
        match sector {
            StrSector::Audio(audio) => {
                assert_eq!(audio.coding.channels, 2);
                assert_eq!(audio.coding.sample_rate, 37800);
                assert_eq!(audio.coding.bits_per_sample, 4);
                assert_eq!(audio.data.len(), STR_FORM2_DATA_SIZE);
            }
            other => panic!("expected an audio sector, got {:?}", other),
        }
    }

    #[test]
    fn a_file_without_a_sync_pattern_is_an_error() {
        let mut bytes = sector(
            submode::DATA,
            0x00,
            &video_user_data(0, 1, 16, [0u8; 8], &[0u8; 16]),
        );
        bytes[2] = 0x00;
        let mut reader = std::io::Cursor::new(bytes);
        assert!(StrSector::read(&mut reader).is_err());
    }

    #[test]
    fn the_end_of_the_file_ends_the_stream() {
        let mut reader = std::io::Cursor::new(Vec::new());
        assert!(StrSector::read(&mut reader).unwrap().is_none());
    }

    #[test]
    fn the_audio_of_a_file_is_decoded_with_its_coding() {
        let mut bytes = sector(
            submode::AUDIO | submode::FORM2 | submode::REAL_TIME,
            0x00,
            &audio_user_data(),
        );
        bytes.extend_from_slice(&sector(
            submode::AUDIO | submode::FORM2 | submode::REAL_TIME,
            0x00,
            &audio_user_data(),
        ));
        let file = StrFile::from_bytes(&bytes).unwrap();

        assert_eq!(file.audio_tracks.len(), 1);
        let track = &file.audio_tracks[0];
        assert_eq!(track.coding.channels, 1);
        assert_eq!(track.samples.len(), 2 * 4032);
        assert!(track.samples.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn a_frame_is_put_together_from_its_sectors() {
        // The frame's bitstream spans the two chunks: 2016 bytes of the first one and 8 of the second.
        let mut first_chunk = vec![0u8; STR_VIDEO_SECTOR_SIZE];
        first_chunk[0..8].copy_from_slice(&[1u8; 8]);
        let mut second_chunk = vec![0u8; STR_VIDEO_SECTOR_SIZE];
        second_chunk[0..8].copy_from_slice(&[2u8; 8]);

        let mut bytes = sector(
            submode::DATA,
            0x00,
            &video_user_data(0, 2, 2024, [1u8; 8], &first_chunk),
        );
        bytes.extend_from_slice(&sector(
            submode::DATA,
            0x00,
            &video_user_data(1, 2, 2024, [1u8; 8], &second_chunk),
        ));
        let file = StrFile::from_bytes(&bytes).unwrap();

        assert_eq!(file.frames.len(), 1);
        let frame = &file.frames[0];
        assert_eq!(frame.number, 1);
        assert_eq!(frame.width, 320);
        assert_eq!(frame.height, 240);
        assert_eq!(frame.bitstream.len(), 2024);
        assert_eq!(frame.bitstream[0..8], [1u8; 8]);
        assert_eq!(frame.bitstream[2016..2024], [2u8; 8]);
    }
}
