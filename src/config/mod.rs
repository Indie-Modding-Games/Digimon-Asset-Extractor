use crate::error::{DwExtractError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub defaults: Defaults,
    pub extensions: HashMap<String, String>,
    #[serde(default)]
    pub files: HashMap<String, String>,
    #[serde(default)]
    pub flatten: Vec<String>,
    #[serde(default = "default_root_folder")]
    pub root_folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub unknown: String,
}

fn default_root_folder() -> String {
    String::new()
}

impl ExtractionConfig {
    pub fn load_default() -> Result<Self> {
        let yaml = include_str!("../../config/extraction_structure.yaml");
        Ok(serde_yaml::from_str(yaml)?)
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn get_output_subdir(&self, filename: &str) -> PathBuf {
        let filename_upper = filename.to_uppercase();

        if let Some(dir) = self.files.get(&filename_upper) {
            return PathBuf::from(dir);
        }

        if let Some(ext) = filename.split('.').last() {
            let ext_lower = ext.to_lowercase();
            if self.flatten.contains(&ext_lower) {
                return PathBuf::new();
            }
            if let Some(dir) = self.extensions.get(&ext_lower) {
                return PathBuf::from(dir);
            }
        }

        PathBuf::from(&self.defaults.unknown)
    }

    pub fn get_full_output_path(&self, base_output: &PathBuf, filename: &str) -> PathBuf {
        let subdir = self.get_output_subdir(filename);
        let mut path = base_output.clone();
        if !self.root_folder.is_empty() {
            path = path.join(&self.root_folder);
        }
        path.join(subdir)
    }
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            defaults: Defaults { unknown: "".to_string() },
            extensions: HashMap::new(),
            files: HashMap::new(),
            flatten: Vec::new(),
            root_folder: String::new(),
        }
    }
}

impl ExtractionConfig {
    pub fn flat() -> Self {
        Self::default()
    }

    pub fn structured() -> Self {
        Self::load_default().unwrap_or_else(|_| Self::flat())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        let config = ExtractionConfig::load_default().unwrap();
        assert_eq!(config.defaults.unknown, "other/");
        assert_eq!(config.extensions.get("tim"), Some(&"textures/".to_string()));
        assert_eq!(config.extensions.get("mmd"), Some(&"models/digimon/".to_string()));
        assert_eq!(config.files.get("SYSTEM.CNF"), Some(&"system/".to_string()));
    }

    #[test]
    fn test_get_output_subdir() {
        let config = ExtractionConfig::load_default().unwrap();
        assert_eq!(config.get_output_subdir("SYSTEM.CNF"), PathBuf::from("system/"));
        assert_eq!(config.get_output_subdir("ALLTIM.TIM"), PathBuf::from("textures/"));
        assert_eq!(config.get_output_subdir("AGUM.MMD"), PathBuf::from("models/digimon/"));
        assert_eq!(config.get_output_subdir("DOOR00.TMD"), PathBuf::from("models/objects/"));
        assert_eq!(config.get_output_subdir("MAYO01.MAP"), PathBuf::from("maps/"));
        assert_eq!(config.get_output_subdir("MAYO01.TFS"), PathBuf::from("maps/"));
        assert_eq!(config.get_output_subdir("EFEDAT.EFE"), PathBuf::from("audio/"));
        assert_eq!(config.get_output_subdir("UNKNOWN.XYZ"), PathBuf::from("other/"));
    }

    #[test]
    fn test_get_full_output_path() {
        let config = ExtractionConfig::load_default().unwrap();
        let base = PathBuf::from("assets");
        assert_eq!(config.get_full_output_path(&base, "AGUM.MMD"), PathBuf::from("assets/models/digimon"));
    }
}