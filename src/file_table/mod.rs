use crate::error::{DwExtractError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub filename: String,
    pub sector: u32,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct FileTable {
    entries: HashMap<String, FileEntry>,
    by_path: HashMap<String, FileEntry>,
}

impl FileTable {
    pub fn load_us() -> Result<Self> {
        let json = include_str!("../../data/file_table_us.json");
        let entries: Vec<FileEntry> = serde_json::from_str(json)?;
        
        let mut by_name = HashMap::new();
        let mut by_path = HashMap::new();
        
        for entry in entries {
            let name_only = entry.filename.split('/').last().unwrap_or(&entry.filename).to_string();
            by_name.insert(name_only.to_uppercase(), entry.clone());
            by_path.insert(entry.filename.to_uppercase(), entry);
        }
        
        Ok(Self { entries: by_name, by_path })
    }

    pub fn get_by_name(&self, name: &str) -> Option<&FileEntry> {
        self.entries.get(&name.to_uppercase())
    }

    pub fn get_by_path(&self, path: &str) -> Option<&FileEntry> {
        self.by_path.get(&path.to_uppercase())
    }

    pub fn get(&self, identifier: &str) -> Option<&FileEntry> {
        self.get_by_path(identifier).or_else(|| self.get_by_name(identifier))
    }

    pub fn all_entries(&self) -> impl Iterator<Item = &FileEntry> {
        self.by_path.values()
    }

    pub fn mmd_files(&self) -> Vec<&FileEntry> {
        self.by_path.values()
            .filter(|e| e.filename.ends_with(".MMD") || e.filename.ends_with(".mmd"))
            .collect()
    }

    pub fn tim_files(&self) -> Vec<&FileEntry> {
        self.by_path.values()
            .filter(|e| e.filename.ends_with(".TIM") || e.filename.ends_with(".tim"))
            .collect()
    }

    pub fn verify_version(&self, bin_reader: &mut crate::bin_parser::BinReader) -> Result<String> {
        let entry = self.get("SYSTEM.CNF")
            .ok_or_else(|| DwExtractError::FileTable("SYSTEM.CNF not in file table".to_string()))?;
        
        let data = bin_reader.read_file_data(entry.sector, entry.size)?;
        let cnf = String::from_utf8_lossy(&data);
        
        // Handle both SLUS-01032 and SLUS_010.32 formats
        if cnf.contains("SLUS-01032") || cnf.contains("SLUS_010.32") || cnf.contains("SLUS_01032") {
            Ok("SLUS-01032".to_string())
        } else if cnf.contains("SLES") {
            Err(DwExtractError::UnsupportedVersion("European version detected (SLES), only SLUS-01032 supported".to_string()))
        } else if cnf.contains("SLPS") {
            Err(DwExtractError::UnsupportedVersion("Japanese version detected (SLPS), only SLUS-01032 supported".to_string()))
        } else {
            Err(DwExtractError::UnsupportedVersion(format!("Unknown version: {}", cnf.trim())))
        }
    }
}

pub fn load_file_table() -> Result<FileTable> {
    FileTable::load_us()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_file_table() {
        let table = FileTable::load_us().unwrap();
        assert!(table.get("SYSTEM.CNF").is_some());
        assert!(table.get("ALLTIM.TIM").is_some());
        assert!(table.get("AGUM.MMD").is_some());
        assert_eq!(table.mmd_files().len(), 178);
        assert_eq!(table.tim_files().len(), 52);
        assert_eq!(table.all_entries().count(), 818);
    }
}