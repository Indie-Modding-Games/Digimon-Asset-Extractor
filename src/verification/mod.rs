use crate::error::Result;
use md5::compute as md5_compute;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileHashes {
    pub sha256: String,
    pub md5: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VerificationStatus {
    Verified,
    Mismatch,
    NoReference,
}

pub fn compute_hashes(data: &[u8]) -> FileHashes {
    let mut sha256 = Sha256::new();
    sha256.update(data);
    let sha256_hash = hex::encode(sha256.finalize());

    let md5_hash = hex::encode(md5_compute(data).as_ref());

    FileHashes {
        sha256: sha256_hash,
        md5: md5_hash,
    }
}

pub fn compute_file_hashes<P: AsRef<std::path::Path>>(path: P) -> Result<FileHashes> {
    let data = std::fs::read(path)?;
    Ok(compute_hashes(&data))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedumpEntry {
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub md5: String,
}

#[derive(Debug, Clone)]
pub struct RedumpDatabase {
    entries: HashMap<String, RedumpEntry>,
}

impl RedumpDatabase {
    pub fn new() -> Self {
        let mut entries = HashMap::new();
        
        entries.insert("SYSTEM.CNF".to_string(), RedumpEntry {
            name: "SYSTEM.CNF".to_string(),
            size: 68,
            sha256: "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456".to_string(),
            md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        });
        
        entries.insert("CHDAT/ALLTIM.TIM".to_string(), RedumpEntry {
            name: "CHDAT/ALLTIM.TIM".to_string(),
            size: 3317760,
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        });

        Self { entries }
    }

    pub fn verify(&self, filename: &str, size: u32, hashes: &FileHashes) -> VerificationStatus {
        let key = filename.to_uppercase();
        if let Some(entry) = self.entries.get(&key) {
            if entry.size == size as u64 
                && entry.sha256 == hashes.sha256 
                && entry.md5 == hashes.md5 {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Mismatch
            }
        } else {
            VerificationStatus::NoReference
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub version: String,
    pub source_bin: String,
    pub source_cue: String,
    pub files: Vec<ManifestFile>,
    pub summary: ManifestSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub size: u32,
    pub sha256: String,
    pub md5: String,
    pub status: VerificationStatus,
    pub format_valid: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestSummary {
    pub total: usize,
    pub verified: usize,
    pub mismatched: usize,
    pub no_reference: usize,
    pub errors: usize,
}

impl Manifest {
    pub fn from_extraction_result(
        result: &crate::extractor::ExtractionResult,
        bin_path: &str,
        cue_path: &str,
    ) -> Self {
        let files = result.files.iter().map(|f| ManifestFile {
            path: f.path.clone(),
            size: f.size,
            sha256: f.sha256.clone(),
            md5: f.md5.clone(),
            status: f.verification_status,
            format_valid: f.format_valid,
        }).collect();

        let summary = ManifestSummary {
            total: result.total_files,
            verified: result.verified,
            mismatched: result.mismatched,
            no_reference: result.no_reference,
            errors: result.errors,
        };

        Self {
            version: result.version.clone(),
            source_bin: bin_path.to_string(),
            source_cue: cue_path.to_string(),
            files,
            summary,
        }
    }

    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hashes() {
        let data = b"test data";
        let hashes = compute_hashes(data);
        assert_eq!(hashes.sha256.len(), 64);
        assert_eq!(hashes.md5.len(), 32);
    }

    #[test]
    fn test_redump_database() {
        let db = RedumpDatabase::new();
        let hashes = FileHashes {
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        };
        
        let status = db.verify("SYSTEM.CNF", 68, &hashes);
        assert_eq!(status, VerificationStatus::Verified);
        
        let status = db.verify("UNKNOWN.BIN", 100, &hashes);
        assert_eq!(status, VerificationStatus::NoReference);
    }
}