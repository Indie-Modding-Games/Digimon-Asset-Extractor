use crate::error::{DwExtractError, Result};
use std::fmt;

pub const EFE_UV_ANIM_SIZE: usize = 0x20;
pub const EFE_SUB_EFFECT_SIZE: usize = 0x1C;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum EfeOpcode {
    Nop = 0x00,
    End = 0x01,
    Call = 0x02,
    Return = 0x03,
    Jump = 0x04,
    JumpIf = 0x05,
    SetReg = 0x06,
    AddReg = 0x07,
    SubReg = 0x08,
    MulReg = 0x09,
    DivReg = 0x0A,
    ModReg = 0x0B,
    AndReg = 0x0C,
    OrReg = 0x0D,
    XorReg = 0x0E,
    ShlReg = 0x0F,
    ShrReg = 0x10,
    NegReg = 0x11,
    NotReg = 0x12,
    CmpReg = 0x13,
    Wait = 0x14,
    Spawn = 0x15,
    Kill = 0x16,
    SetPos = 0x17,
    AddPos = 0x18,
    SetRot = 0x19,
    AddRot = 0x1A,
    SetScale = 0x1B,
    AddScale = 0x1C,
    SetColor = 0x1D,
    AddColor = 0x1E,
    SetAlpha = 0x1F,
    SetTex = 0x20,
    SetUv = 0x21,
    AnimUv = 0x22,
    SetLight = 0x23,
    PlaySfx = 0x24,
    StopSfx = 0x25,
    ShakeCam = 0x26,
    Flash = 0x27,
    Unknown = 0xFFFF,
}

impl From<u16> for EfeOpcode {
    fn from(val: u16) -> Self {
        match val {
            0x00 => EfeOpcode::Nop,
            0x01 => EfeOpcode::End,
            0x02 => EfeOpcode::Call,
            0x03 => EfeOpcode::Return,
            0x04 => EfeOpcode::Jump,
            0x05 => EfeOpcode::JumpIf,
            0x06 => EfeOpcode::SetReg,
            0x07 => EfeOpcode::AddReg,
            0x08 => EfeOpcode::SubReg,
            0x09 => EfeOpcode::MulReg,
            0x0A => EfeOpcode::DivReg,
            0x0B => EfeOpcode::ModReg,
            0x0C => EfeOpcode::AndReg,
            0x0D => EfeOpcode::OrReg,
            0x0E => EfeOpcode::XorReg,
            0x0F => EfeOpcode::ShlReg,
            0x10 => EfeOpcode::ShrReg,
            0x11 => EfeOpcode::NegReg,
            0x12 => EfeOpcode::NotReg,
            0x13 => EfeOpcode::CmpReg,
            0x14 => EfeOpcode::Wait,
            0x15 => EfeOpcode::Spawn,
            0x16 => EfeOpcode::Kill,
            0x17 => EfeOpcode::SetPos,
            0x18 => EfeOpcode::AddPos,
            0x19 => EfeOpcode::SetRot,
            0x1A => EfeOpcode::AddRot,
            0x1B => EfeOpcode::SetScale,
            0x1C => EfeOpcode::AddScale,
            0x1D => EfeOpcode::SetColor,
            0x1E => EfeOpcode::AddColor,
            0x1F => EfeOpcode::SetAlpha,
            0x20 => EfeOpcode::SetTex,
            0x21 => EfeOpcode::SetUv,
            0x22 => EfeOpcode::AnimUv,
            0x23 => EfeOpcode::SetLight,
            0x24 => EfeOpcode::PlaySfx,
            0x25 => EfeOpcode::StopSfx,
            0x26 => EfeOpcode::ShakeCam,
            0x27 => EfeOpcode::Flash,
            _ => EfeOpcode::Unknown,
        }
    }
}

impl fmt::Display for EfeOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EfeOpcode::Nop => write!(f, "nop"),
            EfeOpcode::End => write!(f, "end"),
            EfeOpcode::Call => write!(f, "call"),
            EfeOpcode::Return => write!(f, "ret"),
            EfeOpcode::Jump => write!(f, "jmp"),
            EfeOpcode::JumpIf => write!(f, "jz"),
            EfeOpcode::SetReg => write!(f, "mov"),
            EfeOpcode::AddReg => write!(f, "add"),
            EfeOpcode::SubReg => write!(f, "sub"),
            EfeOpcode::MulReg => write!(f, "mul"),
            EfeOpcode::DivReg => write!(f, "div"),
            EfeOpcode::ModReg => write!(f, "mod"),
            EfeOpcode::AndReg => write!(f, "and"),
            EfeOpcode::OrReg => write!(f, "or"),
            EfeOpcode::XorReg => write!(f, "xor"),
            EfeOpcode::ShlReg => write!(f, "shl"),
            EfeOpcode::ShrReg => write!(f, "shr"),
            EfeOpcode::NegReg => write!(f, "neg"),
            EfeOpcode::NotReg => write!(f, "not"),
            EfeOpcode::CmpReg => write!(f, "cmp"),
            EfeOpcode::Wait => write!(f, "wait"),
            EfeOpcode::Spawn => write!(f, "spawn"),
            EfeOpcode::Kill => write!(f, "kill"),
            EfeOpcode::SetPos => write!(f, "setpos"),
            EfeOpcode::AddPos => write!(f, "addpos"),
            EfeOpcode::SetRot => write!(f, "setrot"),
            EfeOpcode::AddRot => write!(f, "addrot"),
            EfeOpcode::SetScale => write!(f, "setscale"),
            EfeOpcode::AddScale => write!(f, "addscale"),
            EfeOpcode::SetColor => write!(f, "setcol"),
            EfeOpcode::AddColor => write!(f, "addcol"),
            EfeOpcode::SetAlpha => write!(f, "setalp"),
            EfeOpcode::SetTex => write!(f, "settex"),
            EfeOpcode::SetUv => write!(f, "setuv"),
            EfeOpcode::AnimUv => write!(f, "animuv"),
            EfeOpcode::SetLight => write!(f, "setlight"),
            EfeOpcode::PlaySfx => write!(f, "playsfx"),
            EfeOpcode::StopSfx => write!(f, "stopsfx"),
            EfeOpcode::ShakeCam => write!(f, "shakecam"),
            EfeOpcode::Flash => write!(f, "flash"),
            EfeOpcode::Unknown => write!(f, "unk"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EfeInstruction {
    pub offset: usize,
    pub opcode: EfeOpcode,
    pub args: [i32; 3],
    pub raw_opcode: u16,
    pub raw_args: [i16; 3],
}

impl EfeInstruction {
    pub fn read(data: &[u8], offset: usize) -> Result<Self> {
        if offset + 8 > data.len() {
            return Err(DwExtractError::InvalidFormat("Instruction read out of bounds".into()));
        }
        
        let raw_opcode = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let raw_args = [
            i16::from_le_bytes([data[offset + 2], data[offset + 3]]),
            i16::from_le_bytes([data[offset + 4], data[offset + 5]]),
            i16::from_le_bytes([data[offset + 6], data[offset + 7]]),
        ];
        
        Ok(Self {
            offset,
            opcode: EfeOpcode::from(raw_opcode),
            args: [raw_args[0] as i32, raw_args[1] as i32, raw_args[2] as i32],
            raw_opcode,
            raw_args,
        })
    }

    pub fn size() -> usize {
        8
    }

    pub fn disassemble(&self) -> String {
        let opcode_str = self.opcode.to_string();
        match self.opcode {
            EfeOpcode::Nop | EfeOpcode::End | EfeOpcode::Return => {
                opcode_str
            }
            EfeOpcode::Call | EfeOpcode::Jump | EfeOpcode::JumpIf => {
                format!("{} ${:04X}", opcode_str, self.args[0])
            }
            EfeOpcode::Wait => {
                format!("{} {}", opcode_str, self.args[0])
            }
            EfeOpcode::Spawn => {
                format!("{} ${:04X} r{} r{}", opcode_str, self.args[0], self.args[1], self.args[2])
            }
            _ => {
                if self.args[2] != 0 {
                    format!("{} r{}, r{}, r{}", opcode_str, self.args[0], self.args[1], self.args[2])
                } else if self.args[1] != 0 {
                    format!("{} r{}, r{}", opcode_str, self.args[0], self.args[1])
                } else {
                    format!("{} r{}", opcode_str, self.args[0])
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EfeScript {
    pub instructions: Vec<EfeInstruction>,
    pub labels: std::collections::HashMap<usize, String>,
}

impl EfeScript {
    pub fn read(data: &[u8], base_offset: usize) -> Result<Self> {
        let mut instructions = Vec::new();
        let mut offset = 0;
        
        while offset + EfeInstruction::size() <= data.len() {
            let instr = EfeInstruction::read(data, offset)?;
            if instr.opcode == EfeOpcode::End {
                instructions.push(instr);
                break;
            }
            instructions.push(instr);
            offset += EfeInstruction::size();
        }
        
        let mut labels = std::collections::HashMap::new();
        for (_i, instr) in instructions.iter().enumerate() {
            if matches!(instr.opcode, EfeOpcode::Call | EfeOpcode::Jump | EfeOpcode::JumpIf) {
                let target = instr.args[0] as usize;
                if target > base_offset {
                    let rel = target - base_offset;
                    labels.insert(rel, format!("L_{:04X}", rel));
                }
            }
        }
        
        Ok(Self { instructions, labels })
    }

    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        for instr in &self.instructions {
            if let Some(label) = self.labels.get(&instr.offset) {
                out.push_str(&format!("{}:\n", label));
            }
            out.push_str(&format!("  {:04X}: {}\n", instr.offset, instr.disassemble()));
        }
        out
    }
}

pub fn parse_efe_scripts(data: &[u8], base_offset: usize) -> Result<Vec<EfeScript>> {
    let mut scripts = Vec::new();
    let mut offset = 0;
    
    while offset < data.len() {
        if data[offset..].iter().all(|&b| b == 0) {
            break;
        }
        
        let script = EfeScript::read(&data[offset..], base_offset + offset)?;
        let script_size = script.instructions.len() * EfeInstruction::size();
        if script_size == 0 {
            break;
        }
        offset += script_size;
        scripts.push(script);
    }
    
    Ok(scripts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_from() {
        assert_eq!(EfeOpcode::from(0x00), EfeOpcode::Nop);
        assert_eq!(EfeOpcode::from(0x01), EfeOpcode::End);
        assert_eq!(EfeOpcode::from(0x02), EfeOpcode::Call);
        assert_eq!(EfeOpcode::from(0x15), EfeOpcode::Spawn);
        assert_eq!(EfeOpcode::from(0xFFFF), EfeOpcode::Unknown);
    }

    #[test]
    fn test_instruction_read() {
        let data = [
            0x02, 0x00, // Call
            0x10, 0x00, 0x00, 0x00, 0x00, 0x00, // args
        ];
        let instr = EfeInstruction::read(&data, 0).unwrap();
        assert_eq!(instr.opcode, EfeOpcode::Call);
        assert_eq!(instr.args[0], 0x10);
    }

    #[test]
    fn test_instruction_disassemble() {
        let instr = EfeInstruction {
            offset: 0,
            opcode: EfeOpcode::Call,
            args: [0x10, 0, 0],
            raw_opcode: 0x02,
            raw_args: [0x10, 0, 0],
        };
        assert_eq!(instr.disassemble(), "call $0010");
    }
}