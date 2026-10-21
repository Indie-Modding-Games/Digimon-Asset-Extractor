//! The charset and the text of the event scripts.
//!
//! The scripts hold their text in two-byte cells: a lead byte and a trail byte that together are a
//! Shift-JIS (cp932) code. The font of the game replaces a few of them, and the text carries control
//! cells that the game interprets when it prints a line.

use crate::error::{DwExtractError, Result};
use std::sync::OnceLock;

const FIRST_LEAD: u16 = 0x81;
const LAST_LEAD: u16 = 0x84;
/// A lead byte covers two rows of JIS X 0208: trail bytes 0x40-0x7E and 0x80-0xFC.
const FIRST_TRAIL: u8 = 0x40;
const LAST_TRAIL: u8 = 0xFC;
const SKIPPED_TRAIL: u8 = 0x7F;
const CELLS_PER_LEAD: usize = 188;

const FONT_APOSTROPHE: u16 = 0x8175;
const FONT_DOUBLE_QUOTE: u16 = 0x8176;
const CP932_APOSTROPHE: u16 = 0x8166;
const CP932_DOUBLE_QUOTE: u16 = 0x8168;
const RIGHT_SINGLE_QUOTE: char = '\u{2019}';
const RIGHT_DOUBLE_QUOTE: char = '\u{201D}';

const FULL_WIDTH_SPACE: u16 = 0x8140;
const FONT_PERIOD: u16 = 0x8142;
/// Full-width '!'; the block follows ASCII up to '~'.
const FIRST_FULL_WIDTH: u32 = 0xFF01;

const RAW_03: u8 = 0x03;
const NEW_LINE: u8 = 0x0D;
/// Script cells starting below are control cells.
const FIRST_GLYPH_BYTE: u8 = 0x20;

/// cp932 for lead bytes 0x81-0x84, CELLS_PER_LEAD cells each in trail order; 0 is an empty cell.
static NON_KANJI_ROWS: [u16; 752] = [
    0x3000, 0x3001, 0x3002, 0xFF0C, 0xFF0E, 0x30FB, 0xFF1A, 0xFF1B, 0xFF1F, 0xFF01, 0x309B, 0x309C, 0x00B4, 0xFF40,
    0x00A8, 0xFF3E, 0xFFE3, 0xFF3F, 0x30FD, 0x30FE, 0x309D, 0x309E, 0x3003, 0x4EDD, 0x3005, 0x3006, 0x3007, 0x30FC,
    0x2015, 0x2010, 0xFF0F, 0xFF3C, 0xFF5E, 0x2225, 0xFF5C, 0x2026, 0x2025, 0x2018, 0x2019, 0x201C, 0x201D, 0xFF08,
    0xFF09, 0x3014, 0x3015, 0xFF3B, 0xFF3D, 0xFF5B, 0xFF5D, 0x3008, 0x3009, 0x300A, 0x300B, 0x300C, 0x300D, 0x300E,
    0x300F, 0x3010, 0x3011, 0xFF0B, 0xFF0D, 0x00B1, 0x00D7, 0x00F7, 0xFF1D, 0x2260, 0xFF1C, 0xFF1E, 0x2266, 0x2267,
    0x221E, 0x2234, 0x2642, 0x2640, 0x00B0, 0x2032, 0x2033, 0x2103, 0xFFE5, 0xFF04, 0xFFE0, 0xFFE1, 0xFF05, 0xFF03,
    0xFF06, 0xFF0A, 0xFF20, 0x00A7, 0x2606, 0x2605, 0x25CB, 0x25CF, 0x25CE, 0x25C7, 0x25C6, 0x25A1, 0x25A0, 0x25B3,
    0x25B2, 0x25BD, 0x25BC, 0x203B, 0x3012, 0x2192, 0x2190, 0x2191, 0x2193, 0x3013, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x2208, 0x220B, 0x2286, 0x2287, 0x2282, 0x2283, 0x222A,
    0x2229, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x2227, 0x2228, 0xFFE2, 0x21D2, 0x21D4,
    0x2200, 0x2203, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x2220,
    0x22A5, 0x2312, 0x2202, 0x2207, 0x2261, 0x2252, 0x226A, 0x226B, 0x221A, 0x223D, 0x221D, 0x2235, 0x222B, 0x222C,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x212B, 0x2030, 0x266F, 0x266D, 0x266A, 0x2020, 0x2021,
    0x00B6, 0x0000, 0x0000, 0x0000, 0x0000, 0x25EF, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0xFF10, 0xFF11, 0xFF12, 0xFF13, 0xFF14, 0xFF15, 0xFF16,
    0xFF17, 0xFF18, 0xFF19, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0xFF21, 0xFF22, 0xFF23, 0xFF24,
    0xFF25, 0xFF26, 0xFF27, 0xFF28, 0xFF29, 0xFF2A, 0xFF2B, 0xFF2C, 0xFF2D, 0xFF2E, 0xFF2F, 0xFF30, 0xFF31, 0xFF32,
    0xFF33, 0xFF34, 0xFF35, 0xFF36, 0xFF37, 0xFF38, 0xFF39, 0xFF3A, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0xFF41, 0xFF42, 0xFF43, 0xFF44, 0xFF45, 0xFF46, 0xFF47, 0xFF48, 0xFF49, 0xFF4A, 0xFF4B, 0xFF4C, 0xFF4D, 0xFF4E,
    0xFF4F, 0xFF50, 0xFF51, 0xFF52, 0xFF53, 0xFF54, 0xFF55, 0xFF56, 0xFF57, 0xFF58, 0xFF59, 0xFF5A, 0x0000, 0x0000,
    0x0000, 0x0000, 0x3041, 0x3042, 0x3043, 0x3044, 0x3045, 0x3046, 0x3047, 0x3048, 0x3049, 0x304A, 0x304B, 0x304C,
    0x304D, 0x304E, 0x304F, 0x3050, 0x3051, 0x3052, 0x3053, 0x3054, 0x3055, 0x3056, 0x3057, 0x3058, 0x3059, 0x305A,
    0x305B, 0x305C, 0x305D, 0x305E, 0x305F, 0x3060, 0x3061, 0x3062, 0x3063, 0x3064, 0x3065, 0x3066, 0x3067, 0x3068,
    0x3069, 0x306A, 0x306B, 0x306C, 0x306D, 0x306E, 0x306F, 0x3070, 0x3071, 0x3072, 0x3073, 0x3074, 0x3075, 0x3076,
    0x3077, 0x3078, 0x3079, 0x307A, 0x307B, 0x307C, 0x307D, 0x307E, 0x307F, 0x3080, 0x3081, 0x3082, 0x3083, 0x3084,
    0x3085, 0x3086, 0x3087, 0x3088, 0x3089, 0x308A, 0x308B, 0x308C, 0x308D, 0x308E, 0x308F, 0x3090, 0x3091, 0x3092,
    0x3093, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x30A1, 0x30A2,
    0x30A3, 0x30A4, 0x30A5, 0x30A6, 0x30A7, 0x30A8, 0x30A9, 0x30AA, 0x30AB, 0x30AC, 0x30AD, 0x30AE, 0x30AF, 0x30B0,
    0x30B1, 0x30B2, 0x30B3, 0x30B4, 0x30B5, 0x30B6, 0x30B7, 0x30B8, 0x30B9, 0x30BA, 0x30BB, 0x30BC, 0x30BD, 0x30BE,
    0x30BF, 0x30C0, 0x30C1, 0x30C2, 0x30C3, 0x30C4, 0x30C5, 0x30C6, 0x30C7, 0x30C8, 0x30C9, 0x30CA, 0x30CB, 0x30CC,
    0x30CD, 0x30CE, 0x30CF, 0x30D0, 0x30D1, 0x30D2, 0x30D3, 0x30D4, 0x30D5, 0x30D6, 0x30D7, 0x30D8, 0x30D9, 0x30DA,
    0x30DB, 0x30DC, 0x30DD, 0x30DE, 0x30DF, 0x30E0, 0x30E1, 0x30E2, 0x30E3, 0x30E4, 0x30E5, 0x30E6, 0x30E7, 0x30E8,
    0x30E9, 0x30EA, 0x30EB, 0x30EC, 0x30ED, 0x30EE, 0x30EF, 0x30F0, 0x30F1, 0x30F2, 0x30F3, 0x30F4, 0x30F5, 0x30F6,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0391, 0x0392, 0x0393, 0x0394, 0x0395, 0x0396,
    0x0397, 0x0398, 0x0399, 0x039A, 0x039B, 0x039C, 0x039D, 0x039E, 0x039F, 0x03A0, 0x03A1, 0x03A3, 0x03A4, 0x03A5,
    0x03A6, 0x03A7, 0x03A8, 0x03A9, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x03B1, 0x03B2,
    0x03B3, 0x03B4, 0x03B5, 0x03B6, 0x03B7, 0x03B8, 0x03B9, 0x03BA, 0x03BB, 0x03BC, 0x03BD, 0x03BE, 0x03BF, 0x03C0,
    0x03C1, 0x03C3, 0x03C4, 0x03C5, 0x03C6, 0x03C7, 0x03C8, 0x03C9, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0410, 0x0411, 0x0412, 0x0413, 0x0414, 0x0415, 0x0401, 0x0416, 0x0417, 0x0418,
    0x0419, 0x041A, 0x041B, 0x041C, 0x041D, 0x041E, 0x041F, 0x0420, 0x0421, 0x0422, 0x0423, 0x0424, 0x0425, 0x0426,
    0x0427, 0x0428, 0x0429, 0x042A, 0x042B, 0x042C, 0x042D, 0x042E, 0x042F, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0430, 0x0431, 0x0432, 0x0433,
    0x0434, 0x0435, 0x0451, 0x0436, 0x0437, 0x0438, 0x0439, 0x043A, 0x043B, 0x043C, 0x043D, 0x043E, 0x043F, 0x0440,
    0x0441, 0x0442, 0x0443, 0x0444, 0x0445, 0x0446, 0x0447, 0x0448, 0x0449, 0x044A, 0x044B, 0x044C, 0x044D, 0x044E,
    0x044F, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x2500, 0x2502, 0x250C, 0x2510, 0x2518, 0x2514, 0x251C, 0x252C, 0x2524, 0x2534, 0x253C, 0x2501, 0x2503, 0x250F,
    0x2513, 0x251B, 0x2517, 0x2523, 0x2533, 0x252B, 0x253B, 0x254B, 0x2520, 0x252F, 0x2528, 0x2537, 0x253F, 0x251D,
    0x2530, 0x2525, 0x2538, 0x2542, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
];

/// How the parameter of a control cell is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlParameter {
    Value,
    Pstat,
    Raw,
    Ignored,
}

/// A control cell of the script text.
#[derive(Debug, Clone, Copy)]
pub struct ControlTag {
    pub code: u8,
    pub name: &'static str,
    pub parameter: ControlParameter,
}

const CONTROL_TAGS: &[ControlTag] = &[
    ControlTag { code: 0x01, name: "color", parameter: ControlParameter::Value },
    ControlTag { code: 0x02, name: "raw02", parameter: ControlParameter::Raw },
    ControlTag { code: 0x04, name: "num", parameter: ControlParameter::Pstat },
    ControlTag { code: 0x05, name: "tamer", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x06, name: "partner", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x07, name: "digimon", parameter: ControlParameter::Pstat },
    ControlTag { code: 0x08, name: "move", parameter: ControlParameter::Pstat },
    ControlTag { code: 0x09, name: "item", parameter: ControlParameter::Pstat },
    ControlTag { code: 0x0A, name: "bits", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x0B, name: "merit", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x0C, name: "tab96", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x0E, name: "tab132", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x0F, name: "half", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x10, name: "shopvar", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x11, name: "bgm", parameter: ControlParameter::Pstat },
    ControlTag { code: 0x12, name: "tournament", parameter: ControlParameter::Pstat },
    ControlTag { code: 0x13, name: "tourn_won", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x14, name: "tourn_wins", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x15, name: "tourn_lost", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x16, name: "col16", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x17, name: "col17", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x18, name: "col18", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x19, name: "col19", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x1A, name: "col1a", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x1B, name: "col1b", parameter: ControlParameter::Ignored },
    ControlTag { code: 0x1C, name: "col1c", parameter: ControlParameter::Ignored },
];

/// The text of a script and the bytes it takes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptText {
    pub text: String,
    /// The bytes of the run, its final 0D xx 00 yy included.
    pub size: usize,
}

fn cell_of(code: u16) -> Option<usize> {
    let lead = code >> 8;
    let trail = (code & 0xFF) as u8;
    if !(FIRST_LEAD..=LAST_LEAD).contains(&lead)
        || trail < FIRST_TRAIL
        || trail > LAST_TRAIL
        || trail == SKIPPED_TRAIL
    {
        return None;
    }
    let skipped = u32::from(trail > SKIPPED_TRAIL);
    Some(((lead - FIRST_LEAD) as usize * CELLS_PER_LEAD) + (trail - FIRST_TRAIL) as usize - skipped as usize)
}

/// The character of a Shift-JIS code, with the overrides of the game's font.
pub fn sjis_to_unicode(code: u16) -> Option<char> {
    match code {
        FONT_APOSTROPHE => return Some(RIGHT_SINGLE_QUOTE),
        FONT_DOUBLE_QUOTE => return Some(RIGHT_DOUBLE_QUOTE),
        CP932_APOSTROPHE | CP932_DOUBLE_QUOTE => return None,
        _ => {}
    }
    let cell = cell_of(code)?;
    let character = NON_KANJI_ROWS[cell];
    if character == 0 {
        return None;
    }
    char::from_u32(u32::from(character))
}

/// Every mapped character with its code, sorted by character.
fn reverse_table() -> &'static [(char, u16)] {
    static TABLE: OnceLock<Vec<(char, u16)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut pairs = Vec::new();
        for lead in FIRST_LEAD..=LAST_LEAD {
            for trail in FIRST_TRAIL..=LAST_TRAIL {
                let code = (lead << 8) | u16::from(trail);
                if let Some(character) = sjis_to_unicode(code) {
                    pairs.push((character, code));
                }
            }
        }
        pairs.sort_unstable();
        pairs
    })
}

/// The Shift-JIS code of a character. A character the font draws with more than one code takes the lowest one.
pub fn unicode_to_sjis(character: char) -> Option<u16> {
    let table = reverse_table();
    let index = table.partition_point(|probe| probe.0 < character);
    let (found, code) = table.get(index)?;
    if *found == character {
        Some(*code)
    } else {
        None
    }
}

/// The code of an ASCII character in the game's full-width font.
pub fn full_width_of(ascii: char) -> Option<u16> {
    match ascii {
        ' ' => return Some(FULL_WIDTH_SPACE),
        '.' => return Some(FONT_PERIOD),
        '\'' => return Some(FONT_APOSTROPHE),
        '"' => return Some(FONT_DOUBLE_QUOTE),
        _ => {}
    }
    if ascii <= ' ' || ascii > '~' {
        return None;
    }
    unicode_to_sjis(char::from_u32(FIRST_FULL_WIDTH + (ascii as u32 - '!' as u32))?)
}

/// The ASCII character a code stands for, when the font writes it as one.
pub fn ascii_of(code: u16) -> Option<char> {
    match code {
        FULL_WIDTH_SPACE => return Some(' '),
        FONT_PERIOD => return Some('.'),
        FONT_APOSTROPHE => return Some('\''),
        FONT_DOUBLE_QUOTE => return Some('"'),
        _ => {}
    }
    let character = sjis_to_unicode(code)?;
    if (character as u32) < FIRST_FULL_WIDTH || (character as u32) > FIRST_FULL_WIDTH + ('~' as u32 - '!' as u32) {
        return None;
    }
    let ascii = char::from_u32('!' as u32 + (character as u32 - FIRST_FULL_WIDTH))?;
    // The full-width '.' (0x8144) is not the code '.' stands for, so it stays full width.
    if full_width_of(ascii) == Some(code) {
        Some(ascii)
    } else {
        None
    }
}

fn append_character(text: &mut String, character: char) {
    if character == '{' {
        text.push_str("{{");
    } else if character == '}' {
        text.push_str("}}");
    } else {
        text.push(character);
    }
}

/// Decodes the text of a script, the bytes of a run of cells.
pub fn decode_script_text(bytes: &[u8]) -> Result<ScriptText> {
    let mut text = String::new();
    let mut at = 0usize;

    loop {
        if at + 2 > bytes.len() {
            return Err(DwExtractError::InvalidFormat(
                "the text runs past the end of its bytes".into(),
            ));
        }
        let first = bytes[at];
        let second = bytes[at + 1];

        if first == RAW_03 {
            if at + 4 > bytes.len() {
                return Err(DwExtractError::InvalidFormat(
                    "a 0x03 cell runs past the end of the bytes".into(),
                ));
            }
            text.push_str(&format!(
                "{{raw03:{:02X}{:02X}{:02X}}}",
                second, bytes[at + 2], bytes[at + 3]
            ));
            at += 4;
            continue;
        }

        if first == NEW_LINE {
            if at + 2 < bytes.len() && bytes[at + 2] == 0 {
                if at + 4 > bytes.len() {
                    return Err(DwExtractError::InvalidFormat(
                        "the final cells run past the end of the bytes".into(),
                    ));
                }
                let last = bytes[at + 3];
                if second != 0 || last != 0 {
                    text.push_str(&format!("{{end:{:02X}{:02X}}}", second, last));
                }
                return Ok(ScriptText { text, size: at + 4 });
            }
            if second == 0 {
                text.push('\n');
            } else {
                text.push_str(&format!("{{newline:raw={:02X}}}", second));
            }
            at += 2;
            continue;
        }

        if first < FIRST_GLYPH_BYTE {
            match CONTROL_TAGS.iter().find(|tag| tag.code == first) {
                None => text.push_str(&format!("{{cell:{:02X}{:02X}}}", first, second)),
                Some(tag) => match tag.parameter {
                    ControlParameter::Value => text.push_str(&format!("{{{}:{}}}", tag.name, second)),
                    ControlParameter::Pstat => text.push_str(&format!("{{{}:pstat={}}}", tag.name, second)),
                    ControlParameter::Raw => text.push_str(&format!("{{{}:{:02X}}}", tag.name, second)),
                    ControlParameter::Ignored if second != 0 => {
                        text.push_str(&format!("{{{}:raw={:02X}}}", tag.name, second));
                    }
                    ControlParameter::Ignored => text.push_str(&format!("{{{}}}", tag.name)),
                },
            }
            at += 2;
            continue;
        }

        let code = (u16::from(first) << 8) | u16::from(second);
        if let Some(ascii) = ascii_of(code) {
            append_character(&mut text, ascii);
        } else if let Some(character) = sjis_to_unicode(code) {
            append_character(&mut text, character);
        } else {
            text.push_str(&format!("{{cell:{:04X}}}", code));
        }
        at += 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_full_width_font_writes_the_ascii_letters() {
        assert_eq!(ascii_of(0x8260), Some('A'));
        assert_eq!(ascii_of(0x8140), Some(' '));
        assert_eq!(ascii_of(0x8142), Some('.'));
        assert_eq!(ascii_of(0x8175), Some('\''));
        assert_eq!(ascii_of(0x8176), Some('"'));
        // The full-width period is not the code the period stands for.
        assert_eq!(ascii_of(0x8144), None);
    }

    #[test]
    fn the_font_replaces_the_apostrophe_and_the_double_quote() {
        assert_eq!(sjis_to_unicode(0x8175), Some('\u{2019}'));
        assert_eq!(sjis_to_unicode(0x8176), Some('\u{201D}'));
        assert_eq!(sjis_to_unicode(0x8166), None);
        assert_eq!(sjis_to_unicode(0x8168), None);
    }

    #[test]
    fn a_character_and_its_code_round_trip() {
        for ascii in '!'..='~' {
            let code = full_width_of(ascii).expect("the font writes every printable ASCII");
            assert_eq!(ascii_of(code), Some(ascii), "{:?}", ascii);
        }
    }

    #[test]
    fn a_run_of_text_ends_with_its_four_bytes() {
        let text = decode_script_text(&[0x82, 0x60, 0x0D, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(text.text, "A");
        assert_eq!(text.size, 6);
    }

    #[test]
    fn a_text_that_does_not_end_is_an_error() {
        assert!(decode_script_text(&[0x82, 0x60]).is_err());
    }

    #[test]
    fn the_control_cells_are_tags() {
        let text = decode_script_text(&[0x01, 0x03, 0x0D, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(text.text, "{color:3}");

        let text = decode_script_text(&[0x04, 0x07, 0x0D, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(text.text, "{num:pstat=7}");
    }
}
