use crate::error::Result;
use md5::compute as md5_compute;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub mod sha1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileHashes {
    pub sha256: String,
    pub sha1: String,
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

    let sha1_hash = hex::encode(sha1::sha1(data));

    FileHashes {
        sha256: sha256_hash,
        sha1: sha1_hash,
        md5: md5_hash,
    }
}

pub fn compute_file_hashes<P: AsRef<std::path::Path>>(path: P) -> Result<FileHashes> {
    let data = std::fs::read(path)?;
    Ok(compute_hashes(&data))
}

/// A reference entry of a Redump-style list: the expected size and the expected hashes.
/// The hashes that the list does not provide stay empty and are not compared.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedumpEntry {
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub sha1: String,
    pub md5: String,
}

impl RedumpEntry {
    /// Builds an entry from a single hash. The algorithm is chosen by the length of the
    /// hash: 32 hexadecimal characters are MD5, 40 are SHA-1 and 64 are SHA-256.
    pub fn new(name: &str, size: u64, hash: &str) -> Self {
        let mut entry = RedumpEntry {
            name: name.to_string(),
            size,
            sha256: String::new(),
            sha1: String::new(),
            md5: String::new(),
        };

        match hash.len() {
            32 => entry.md5 = hash.to_lowercase(),
            40 => entry.sha1 = hash.to_lowercase(),
            64 => entry.sha256 = hash.to_lowercase(),
            _ => {}
        }

        entry
    }

    /// True when the entry carries no hash at all and only the size can be compared.
    pub fn has_hash(&self) -> bool {
        !self.sha256.is_empty() || !self.sha1.is_empty() || !self.md5.is_empty()
    }
}

/// The set of reference hashes used to check the extracted files.
///
/// The database starts empty: it is only filled from a real reference list, never with
/// placeholder values. Without a list every file is reported as `NoReference`.
#[derive(Debug, Clone, Default)]
pub struct RedumpDatabase {
    entries: HashMap<String, RedumpEntry>,
}

impl RedumpDatabase {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn insert(&mut self, entry: RedumpEntry) {
        self.entries.insert(entry.name.to_uppercase(), entry);
    }

    /// Parses a Redump-style list: one entry per line as `hash,size,filename` or
    /// `hash size filename`. Lines that do not start with a hexadecimal hash (headers,
    /// comments) are ignored.
    pub fn from_list_text(text: &str) -> Result<Self> {
        let mut database = Self::new();

        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let fields: Vec<&str> = if line.contains(',') {
                line.split(',').map(|field| field.trim()).collect()
            } else {
                line.split_whitespace().collect()
            };

            if fields.len() < 3 {
                continue;
            }

            let hash = fields[0].to_lowercase();
            if !is_hex(&hash) {
                continue;
            }

            let size: u64 = match fields[1].parse() {
                Ok(size) => size,
                Err(_) => {
                    return Err(crate::error::DwExtractError::FileTable(format!(
                        "Invalid size in reference list line {}: {}",
                        number + 1,
                        fields[1]
                    )))
                }
            };

            let name = fields[2..].join(" ").trim().trim_matches('"').to_string();
            if name.is_empty() {
                continue;
            }

            let entry = RedumpEntry::new(&name, size, &hash);
            if !entry.has_hash() {
                return Err(crate::error::DwExtractError::FileTable(format!(
                    "Unknown hash length in reference list line {}: {}",
                    number + 1,
                    fields[0]
                )));
            }

            database.insert(entry);
        }

        Ok(database)
    }

    /// Loads a Redump-style list from a file.
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::from_list_text(&text)
    }

    pub fn verify(&self, filename: &str, size: u32, hashes: &FileHashes) -> VerificationStatus {
        let key = filename.to_uppercase();
        let entry = match self.entries.get(&key) {
            Some(entry) => entry,
            None => return VerificationStatus::NoReference,
        };

        if entry.size != size as u64 {
            return VerificationStatus::Mismatch;
        }

        if !entry.sha256.is_empty() && entry.sha256 != hashes.sha256 {
            return VerificationStatus::Mismatch;
        }

        if !entry.sha1.is_empty() && entry.sha1 != hashes.sha1 {
            return VerificationStatus::Mismatch;
        }

        if !entry.md5.is_empty() && entry.md5 != hashes.md5 {
            return VerificationStatus::Mismatch;
        }

        VerificationStatus::Verified
    }
}

fn is_hex(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_hexdigit())
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

    const EMPTY_MD5: &str = "d41d8cd98f00b204e9800998ecf8427e";
    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn test_compute_hashes() {
        let data = b"test data";
        let hashes = compute_hashes(data);
        assert_eq!(hashes.sha256.len(), 64);
        assert_eq!(hashes.sha1.len(), 40);
        assert_eq!(hashes.md5.len(), 32);
    }

    #[test]
    fn test_redump_database_starts_empty() {
        let db = RedumpDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);

        let hashes = compute_hashes(b"test data");
        assert_eq!(
            db.verify("SYSTEM.CNF", 10, &hashes),
            VerificationStatus::NoReference
        );
    }

    #[test]
    fn test_redump_database_has_no_placeholder_hashes() {
        let db = RedumpDatabase::new();
        for entry in db.entries.values() {
            assert_ne!(entry.md5, EMPTY_MD5, "placeholder MD5 in {}", entry.name);
            assert_ne!(entry.sha256, EMPTY_SHA256, "placeholder SHA256 in {}", entry.name);
            assert!(entry.has_hash(), "entry without hash: {}", entry.name);
        }
    }

    #[test]
    fn test_redump_verify_with_real_entry() {
        let test_data = b"test data";
        let hashes = compute_hashes(test_data);

        let mut db = RedumpDatabase::new();
        db.insert(RedumpEntry {
            name: "TEST.BIN".to_string(),
            size: test_data.len() as u64,
            sha256: hashes.sha256.clone(),
            sha1: hashes.sha1.clone(),
            md5: hashes.md5.clone(),
        });
        assert_eq!(
            db.verify("TEST.BIN", test_data.len() as u32, &hashes),
            VerificationStatus::Verified
        );
        assert_eq!(
            db.verify("test.bin", test_data.len() as u32, &hashes),
            VerificationStatus::Verified
        );
        assert_eq!(
            db.verify("TEST.BIN", 999, &hashes),
            VerificationStatus::Mismatch
        );
        assert_eq!(
            db.verify("UNKNOWN.BIN", test_data.len() as u32, &hashes),
            VerificationStatus::NoReference
        );

        let other = compute_hashes(b"other data");
        assert_eq!(
            db.verify("TEST.BIN", test_data.len() as u32, &other),
            VerificationStatus::Mismatch
        );
    }

    #[test]
    fn test_redump_list_parsing() {
        let list = "\
# a comment
hash,size,filename
d41d8cd98f00b204e9800998ecf8427e,0,EMPTY.BIN
a9993e364706816aba3e25717850c26c9cd0d89d,3,ABC.BIN
a9993e364706816aba3e25717850c26c9cd0d89d 3 SPACE.BIN
";

        let db = RedumpDatabase::from_list_text(list).unwrap();
        assert_eq!(db.len(), 3);

        let hashes = compute_hashes(b"");
        assert_eq!(
            db.verify("EMPTY.BIN", 0, &hashes),
            VerificationStatus::Verified
        );

        let abc = compute_hashes(b"abc");
        assert_eq!(db.verify("ABC.BIN", 3, &abc), VerificationStatus::Verified);
        assert_eq!(
            db.verify("SPACE.BIN", 3, &abc),
            VerificationStatus::Verified
        );
    }

    #[test]
    fn test_redump_list_parsing_rejects_bad_size() {
        let list = "a9993e364706816aba3e25717850c26c9cd0d89d,notanumber,ABC.BIN";
        assert!(RedumpDatabase::from_list_text(list).is_err());
    }

    #[test]
    fn test_redump_list_parsing_rejects_unknown_hash_length() {
        let list = "abcd,3,ABC.BIN";
        assert!(RedumpDatabase::from_list_text(list).is_err());
    }
}