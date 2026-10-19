use crate::error::{DwExtractError, Result};
use crate::file_table::{FileEntry, FileTable};
use crate::formats::{MmdInfo, TimHeader};
use crate::verification::{compute_hashes, VerificationStatus};
use std::fs;
use std::path::PathBuf;

pub struct Extractor {
    file_table: FileTable,
    output_dir: PathBuf,
    force_overwrite: bool,
    verify_formats: bool,
}

impl Extractor {
    pub fn new(output_dir: PathBuf, force_overwrite: bool, verify_formats: bool) -> Result<Self> {
        let file_table = FileTable::load_us()?;
        
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)?;
        }
        
        Ok(Self {
            file_table,
            output_dir,
            force_overwrite,
            verify_formats,
        })
    }

    pub fn extract_all(&self, bin_reader: &mut crate::bin_parser::BinReader) -> Result<ExtractionResult> {
        let version = self.file_table.verify_version(bin_reader)?;
        println!("Detected version: {}", version);
        
        let mut results = Vec::new();
        let mut verified = 0;
        let mut mismatched = 0;
        let mut no_ref = 0;
        let mut errors = 0;

        for entry in self.file_table.all_entries() {
            match self.extract_file(bin_reader, entry) {
                Ok(result) => {
                    match result.verification_status {
                        VerificationStatus::Verified => verified += 1,
                        VerificationStatus::Mismatch => mismatched += 1,
                        VerificationStatus::NoReference => no_ref += 1,
                    }
                    results.push(result);
                }
                Err(e) => {
                    errors += 1;
                    eprintln!("Error extracting {}: {}", entry.filename, e);
                    results.push(ExtractionFileResult {
                        path: entry.filename.clone(),
                        size: entry.size,
                        sha256: String::new(),
                        md5: String::new(),
                        verification_status: VerificationStatus::Mismatch,
                        format_valid: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(ExtractionResult {
            version,
            total_files: results.len(),
            verified,
            mismatched,
            no_reference: no_ref,
            errors,
            files: results,
        })
    }

    pub fn extract_selected(&self, bin_reader: &mut crate::bin_parser::BinReader, names: &[String]) -> Result<ExtractionResult> {
        let version = self.file_table.verify_version(bin_reader)?;
        println!("Detected version: {}", version);
        
        let mut results = Vec::new();
        let mut verified = 0;
        let mut mismatched = 0;
        let mut no_ref = 0;
        let mut errors = 0;

        for name in names {
            let entry = self.file_table.get(name)
                .ok_or_else(|| DwExtractError::FileNotFound(name.clone()))?;
            
            match self.extract_file(bin_reader, entry) {
                Ok(result) => {
                    match result.verification_status {
                        VerificationStatus::Verified => verified += 1,
                        VerificationStatus::Mismatch => mismatched += 1,
                        VerificationStatus::NoReference => no_ref += 1,
                    }
                    results.push(result);
                }
                Err(e) => {
                    errors += 1;
                    eprintln!("Error extracting {}: {}", name, e);
                    results.push(ExtractionFileResult {
                        path: entry.filename.clone(),
                        size: entry.size,
                        sha256: String::new(),
                        md5: String::new(),
                        verification_status: VerificationStatus::Mismatch,
                        format_valid: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(ExtractionResult {
            version,
            total_files: results.len(),
            verified,
            mismatched,
            no_reference: no_ref,
            errors,
            files: results,
        })
    }

    fn extract_file(&self, bin_reader: &mut crate::bin_parser::BinReader, entry: &FileEntry) -> Result<ExtractionFileResult> {
        let output_path = self.output_dir.join(&entry.filename);
        
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = bin_reader.read_file_data(entry.sector, entry.size)?;
        
        let hashes = compute_hashes(&data);
        
        let should_write = if output_path.exists() {
            if self.force_overwrite {
                true
            } else {
                let existing_data = fs::read(&output_path)?;
                let existing_hashes = compute_hashes(&existing_data);
                if existing_hashes.sha256 == hashes.sha256 {
                    println!("  {} (unchanged, skipping)", entry.filename);
                    return Ok(ExtractionFileResult {
                        path: entry.filename.clone(),
                        size: entry.size,
                        sha256: hashes.sha256,
                        md5: hashes.md5,
                        verification_status: VerificationStatus::NoReference,
                        format_valid: true,
                        error: None,
                    });
                } else {
                    return Err(DwExtractError::OutputExists(format!(
                        "{} exists with different content. Use --force to overwrite.", output_path.display()
                    )));
                }
            }
        } else {
            true
        };

        if should_write {
            fs::write(&output_path, &data)?;
            println!("  Extracted: {} ({} bytes)", entry.filename, entry.size);
        }

        let format_valid = if self.verify_formats {
            self.verify_format(&entry.filename, &data)?
        } else {
            true
        };

        let verification_status = VerificationStatus::NoReference;

        Ok(ExtractionFileResult {
            path: entry.filename.clone(),
            size: entry.size,
            sha256: hashes.sha256,
            md5: hashes.md5,
            verification_status,
            format_valid,
            error: None,
        })
    }

    fn verify_format(&self, filename: &str, data: &[u8]) -> Result<bool> {
        let upper = filename.to_uppercase();
        
        if upper.ends_with(".TIM") {
            match TimHeader::validate(data) {
                Ok(info) => {
                    println!("    TIM: {}bpp, {}x{}, CLUT: {}", info.bpp, info.width, info.height, info.has_clut);
                    Ok(true)
                }
                Err(e) => {
                    eprintln!("    TIM validation failed: {}", e);
                    Ok(false)
                }
            }
        } else if upper.ends_with(".MMD") {
            match MmdInfo::validate(data) {
                Ok(info) => {
                    println!("    MMD: {} objects, texture: {}", info.tmd_header.n_obj, info.texture_info.is_some());
                    Ok(true)
                }
                Err(e) => {
                    eprintln!("    MMD validation failed: {}", e);
                    Ok(false)
                }
            }
        } else {
            Ok(true)
        }
    }

    pub fn list_files(&self) -> Vec<&FileEntry> {
        self.file_table.all_entries().collect()
    }

    pub fn list_mmd_files(&self) -> Vec<&FileEntry> {
        self.file_table.mmd_files()
    }

    pub fn list_tim_files(&self) -> Vec<&FileEntry> {
        self.file_table.tim_files()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtractionResult {
    pub version: String,
    pub total_files: usize,
    pub verified: usize,
    pub mismatched: usize,
    pub no_reference: usize,
    pub errors: usize,
    pub files: Vec<ExtractionFileResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtractionFileResult {
    pub path: String,
    pub size: u32,
    pub sha256: String,
    pub md5: String,
    pub verification_status: VerificationStatus,
    pub format_valid: bool,
    pub error: Option<String>,
}

impl ExtractionResult {
    pub fn print_summary(&self) {
        println!("\n=== Extraction Summary ===");
        println!("Version: {}", self.version);
        println!("Total files: {}", self.total_files);
        println!("Verified: {}", self.verified);
        println!("Mismatched: {}", self.mismatched);
        println!("No reference: {}", self.no_reference);
        println!("Errors: {}", self.errors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extractor_creation() {
        let dir = tempdir().unwrap();
        let extractor = Extractor::new(dir.path().to_path_buf(), false, false).unwrap();
        assert_eq!(extractor.list_files().len(), 57);
        assert_eq!(extractor.list_mmd_files().len(), 54);
        assert_eq!(extractor.list_tim_files().len(), 1);
    }
}