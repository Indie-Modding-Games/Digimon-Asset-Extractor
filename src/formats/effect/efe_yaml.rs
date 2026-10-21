use crate::error::{DwExtractError, Result};
use crate::formats::effect::{EfeFile, EfeHeader, EfeOpcode};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct EfeYamlFile {
    pub slots: Vec<EfeYamlSlot>,
}

#[derive(Debug, Serialize)]
pub struct EfeYamlSlot {
    pub index: usize,
    pub effect_id: i32,
    pub header: EfeYamlHeader,
    pub is_empty: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<EfeYamlScript>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tim: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EfeYamlHeader {
    pub start_script: i32,
    pub init_script: i32,
    pub uv_anims: i32,
    pub uv_anims_end: i32,
    pub sub_effects: i32,
    pub tmd_start: i32,
    pub tmd_end: i32,
    pub tim_start: i32,
    pub tim_end: i32,
    pub data_end: i32,
    pub spawn_count: i32,
    pub num_sub_effects: i32,
    pub effect_id: i32,
}

#[derive(Debug, Serialize)]
pub struct EfeYamlScript {
    pub name: String,
    pub instructions: Vec<EfeYamlInstruction>,
}

#[derive(Debug, Serialize)]
pub struct EfeYamlInstruction {
    pub offset: usize,
    pub opcode: String,
    pub args: [i32; 3],
    #[serde(skip_serializing_if = "is_default")]
    pub label: Option<String>,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

impl From<&EfeHeader> for EfeYamlHeader {
    fn from(h: &EfeHeader) -> Self {
        Self {
            start_script: h.start_script,
            init_script: h.init_script,
            uv_anims: h.uv_anims,
            uv_anims_end: h.uv_anims_end,
            sub_effects: h.sub_effects,
            tmd_start: h.tmd_start,
            tmd_end: h.tmd_end,
            tim_start: h.tim_start,
            tim_end: h.tim_end,
            data_end: h.data_end,
            spawn_count: h.spawn_count,
            num_sub_effects: h.num_sub_effects,
            effect_id: h.effect_id,
        }
    }
}

pub fn efe_file_to_yaml(efe: &EfeFile, extract_assets: bool) -> Result<EfeYamlFile> {
    let mut yaml_slots = Vec::new();
    
    for slot in &efe.slots {
        if slot.is_empty {
            yaml_slots.push(EfeYamlSlot {
                index: slot.index,
                effect_id: slot.header.effect_id,
                header: (&slot.header).into(),
                is_empty: true,
                scripts: Vec::new(),
                tmd: None,
                tim: None,
            });
            continue;
        }
        
        let scripts_data = slot.scripts_data();
        let scripts = crate::formats::effect::efe_script::parse_efe_scripts(scripts_data, slot.header.uv_anims as usize)?;
        
        let yaml_scripts: Vec<EfeYamlScript> = scripts.iter().enumerate().map(|(i, s)| {
            let mut labels: HashMap<usize, String> = HashMap::new();
            for instr in &s.instructions {
                if matches!(instr.opcode, EfeOpcode::Call | EfeOpcode::Jump | EfeOpcode::JumpIf) {
                    let target = instr.args[0] as usize;
                    if target > instr.offset {
                        labels.insert(target - s.instructions[0].offset, format!("L_{:04X}", target - s.instructions[0].offset));
                    }
                }
            }
            
            EfeYamlScript {
                name: format!("script_{}", i),
                instructions: s.instructions.iter().map(|instr| {
                    let label = labels.get(&instr.offset).cloned();
                    EfeYamlInstruction {
                        offset: instr.offset,
                        opcode: instr.opcode.to_string(),
                        args: instr.args,
                        label,
                    }
                }).collect(),
            }
        }).collect();
        
        let tmd_data = slot.tmd_data();
        let tim_data = slot.tim_data();
        
        yaml_slots.push(EfeYamlSlot {
            index: slot.index,
            effect_id: slot.header.effect_id,
            header: (&slot.header).into(),
            is_empty: false,
            scripts: yaml_scripts,
            tmd: if extract_assets && !tmd_data.is_empty() { Some("extracted".to_string()) } else { None },
            tim: if extract_assets && !tim_data.is_empty() { Some("extracted".to_string()) } else { None },
        });
    }
    
    Ok(EfeYamlFile { slots: yaml_slots })
}

pub fn write_efe_yaml<P: AsRef<std::path::Path>>(path: P, efe: &EfeFile, extract_assets: bool) -> Result<()> {
    let yaml = efe_file_to_yaml(efe, extract_assets)?;
    let yaml_str = serde_yaml::to_string(&yaml)
        .map_err(|e| DwExtractError::Yaml(e))?;
    std::fs::write(path, yaml_str)?;
    Ok(())
}

pub fn extract_efe_assets<P: AsRef<std::path::Path>>(efe: &EfeFile, output_dir: P) -> Result<()> {
    use crate::formats::tim::TimHeader;
    use crate::formats::mmd::MmdInfo;
    
    std::fs::create_dir_all(&output_dir)?;
    
    for slot in efe.used_slots() {
        let tmd_data = slot.tmd_data();
        let tim_data = slot.tim_data();
        
        if !tmd_data.is_empty() {
            let tmd_path = output_dir.as_ref().join(format!("effect_{:03}_slot_{:03}.tmd", slot.header.effect_id, slot.index));
            std::fs::write(&tmd_path, tmd_data)?;
            if let Ok(_) = MmdInfo::validate(tmd_data) {
                println!("  Extracted TMD: {}", tmd_path.display());
            }
        }
        
        if !tim_data.is_empty() {
            let tim_path = output_dir.as_ref().join(format!("effect_{:03}_slot_{:03}.tim", slot.header.effect_id, slot.index));
            std::fs::write(&tim_path, tim_data)?;
            if let Ok(_) = TimHeader::validate(tim_data) {
                println!("  Extracted TIM: {}", tim_path.display());
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efe_yaml_header_from() {
        let header = EfeHeader {
            start_script: 0x10,
            init_script: 0x20,
            uv_anims: 0x100,
            uv_anims_end: 0x200,
            sub_effects: 0x300,
            tmd_start: 0x400,
            tmd_end: 0x500,
            tim_start: 0x600,
            tim_end: 0x700,
            data_end: 0x800,
            spawn_count: 5,
            num_sub_effects: 3,
            effect_id: 0x101,
        };
        let yaml_header: EfeYamlHeader = (&header).into();
        assert_eq!(yaml_header.effect_id, 0x101);
        assert_eq!(yaml_header.spawn_count, 5);
    }
}