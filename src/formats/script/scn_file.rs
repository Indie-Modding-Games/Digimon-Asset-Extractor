//! The container of the event scripts.
//!
//! A file of scripts starts with a table of 0x2000 bytes: the offsets of the scripts, as u32 little endian. The
//! last entry of the table is the size of the file, and the scripts are the ranges between an entry and the next
//! one. Each script starts with its own header: where its code starts, and the sections it holds.

use crate::error::{DwExtractError, Result};

/// The size of the table of offsets at the start of a file of scripts.
pub const SCRIPT_TABLE_SIZE: usize = 0x2000;
/// The id of the end of the list of sections.
pub const SECTION_LIST_END: u16 = 0xFFFF;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptSlot {
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptSection {
    pub id: u16,
    pub offset: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptHeader {
    pub code_start: u16,
    pub sections: Vec<ScriptSection>,
}

fn u16_at(data: &[u8], at: usize) -> Result<u16> {
    if at + 2 > data.len() {
        return Err(DwExtractError::InvalidFormat(
            "the script runs past the end of the file".into(),
        ));
    }
    Ok(u16::from_le_bytes([data[at], data[at + 1]]))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    if at + 4 > data.len() {
        return Err(DwExtractError::InvalidFormat(
            "the table of scripts runs past the end of the file".into(),
        ));
    }
    Ok(u32::from_le_bytes([
        data[at],
        data[at + 1],
        data[at + 2],
        data[at + 3],
    ]))
}

/// Reads the table of offsets: the scripts are the ranges between an entry and the next one, and the last entry
/// is the size of the file.
pub fn read_script_table(data: &[u8]) -> Result<Vec<ScriptSlot>> {
    if data.len() < SCRIPT_TABLE_SIZE {
        return Err(DwExtractError::InvalidFormat(format!(
            "file has {} bytes, less than the {} byte table of scripts",
            data.len(),
            SCRIPT_TABLE_SIZE
        )));
    }

    let file_size = data.len() as u32;
    let mut slots: Vec<ScriptSlot> = Vec::new();
    let mut current = u32_at(data, 0)?;

    if current < SCRIPT_TABLE_SIZE as u32 || current > file_size {
        return Err(DwExtractError::InvalidFormat(format!(
            "script 0 starts at {:#x}, outside the code that follows the table",
            current
        )));
    }

    let mut entry = 1;
    loop {
        let next = u32_at(data, entry * 4)?;

        if next == file_size {
            slots.push(ScriptSlot {
                offset: current,
                size: next - current,
            });
            break;
        }

        if next < current {
            return Err(DwExtractError::InvalidFormat(format!(
                "script {} starts at {:#x}, before the script that ends there, at {:#x}",
                slots.len(),
                next,
                current
            )));
        }
        if next > file_size {
            return Err(DwExtractError::InvalidFormat(format!(
                "script {} ends at {:#x}, past the end of the file, at {:#x}",
                slots.len(),
                next,
                file_size
            )));
        }

        slots.push(ScriptSlot {
            offset: current,
            size: next - current,
        });
        current = next;
        entry += 1;
    }

    Ok(slots)
}

/// Reads the header of a script: where its code starts, and the sections it holds.
pub fn read_script_header(script: &[u8]) -> Result<ScriptHeader> {
    let code_start = u16_at(script, 0)?;
    let mut sections: Vec<ScriptSection> = Vec::new();
    let mut at = 2usize;

    loop {
        let id = u16_at(script, at)?;
        at += 2;
        if id == SECTION_LIST_END {
            break;
        }
        let offset = u16_at(script, at)?;
        at += 2;
        sections.push(ScriptSection { id, offset });
    }

    if code_start as usize != at {
        return Err(DwExtractError::InvalidFormat(format!(
            "code starts at {:#x}, not after the list of sections, at {:#x}",
            code_start, at
        )));
    }

    for section in &sections {
        if section.offset < code_start || section.offset as usize >= script.len() {
            return Err(DwExtractError::InvalidFormat(format!(
                "section {:#x} starts at {:#x}, outside the code of the script",
                section.id, section.offset
            )));
        }
    }

    Ok(ScriptHeader { code_start, sections })
}

/// Reads the table of scripts, and the header of the first one.
pub fn parse_scn_file(data: &[u8]) -> Result<(Vec<ScriptSlot>, ScriptHeader)> {
    let slots = read_script_table(data)?;
    let first = slots.first().ok_or_else(|| {
        DwExtractError::InvalidFormat("the file of scripts holds no script".into())
    })?;
    let header = read_script_header(&data[first.offset as usize..])?;
    Ok((slots, header))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_with_scripts(scripts: &[&[u8]]) -> Vec<u8> {
        let mut offsets = Vec::new();
        let mut body = vec![0u8; SCRIPT_TABLE_SIZE];
        let mut code = Vec::new();
        for script in scripts {
            offsets.push((SCRIPT_TABLE_SIZE + code.len()) as u32);
            code.extend_from_slice(script);
        }
        offsets.push((SCRIPT_TABLE_SIZE + code.len()) as u32);
        for (index, offset) in offsets.iter().enumerate() {
            body[index * 4..index * 4 + 4].copy_from_slice(&offset.to_le_bytes());
        }
        body.extend_from_slice(&code);
        body
    }

    #[test]
    fn the_table_holds_the_scripts_as_ranges() {
        let script = [0x04, 0x00, 0xFF, 0xFF];
        let file = file_with_scripts(&[&script]);
        let slots = read_script_table(&file).unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].offset as usize, SCRIPT_TABLE_SIZE);
        assert_eq!(slots[0].size as usize, script.len());
    }

    #[test]
    fn a_table_without_its_terminator_is_an_error() {
        let mut file = file_with_scripts(&[&[0x04, 0x00, 0xFF, 0xFF]]);
        // No entry of the table is the size of the file, so the table never ends.
        for entry in 0..SCRIPT_TABLE_SIZE / 4 {
            let value = 0x2000u32;
            file[entry * 4..entry * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        assert!(read_script_table(&file).is_err());
    }

    #[test]
    fn the_header_holds_the_sections() {
        let mut script = Vec::new();
        script.extend_from_slice(&8u16.to_le_bytes());
        script.extend_from_slice(&1u16.to_le_bytes());
        script.extend_from_slice(&8u16.to_le_bytes());
        script.extend_from_slice(&0xFFFFu16.to_le_bytes());
        script.extend_from_slice(&[0u8; 4]);

        let header = read_script_header(&script).unwrap();
        assert_eq!(header.code_start, 8);
        assert_eq!(header.sections.len(), 1);
        assert_eq!(header.sections[0], ScriptSection { id: 1, offset: 8 });
    }

    #[test]
    fn a_header_whose_code_does_not_start_after_the_sections_is_an_error() {
        let mut script = Vec::new();
        script.extend_from_slice(&9u16.to_le_bytes());
        script.extend_from_slice(&0xFFFFu16.to_le_bytes());
        script.extend_from_slice(&[0u8; 4]);
        assert!(read_script_header(&script).is_err());
    }
}
