use crate::config::ExtractionConfig;
use crate::error::{DwExtractError, Result};
use crate::file_table::{FileEntry, FileTable};
use crate::formats::{MmdInfo, TimHeader};
use crate::verification::{compute_hashes, RedumpDatabase, VerificationStatus};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CHDAT_MMD_MAPPING: &[(&str, &str)] = &[
    ("AGUM.MMD", "MMD0"), ("AIRD.MMD", "MMD0"), ("ANGE.MMD", "MMD0"), ("BETA.MMD", "MMD0"), ("BIRD.MMD", "MMD0"), ("BOTA.MMD", "MMD0"), ("BOYS.MMD", "MMD0"), ("DEVI.MMD", "MMD0"), ("ELEC.MMD", "MMD0"), ("GABU.MMD", "MMD0"), ("GARU.MMD", "MMD0"), ("GREY.MMD", "MMD0"), ("HOEE.MMD", "MMD0"), ("KABU.MMD", "MMD0"), ("KORO.MMD", "MMD0"), ("MAME.MMD", "MMD0"), ("MERA.MMD", "MMD0"), ("MONZ.MMD", "MMD0"), ("MTGR.MMD", "MMD0"), ("MTMA.MMD", "MMD0"), ("NUME.MMD", "MMD0"), ("POYO.MMD", "MMD0"), ("PUNI.MMD", "MMD0"), ("SEAD.MMD", "MMD0"), ("SKUL.MMD", "MMD0"), ("TUNO.MMD", "MMD0"), ("TYRA.MMD", "MMD0"), ("VEDA.MMD", "MMD0"), ("VEGI.MMD", "MMD0"), ("YUKI.MMD", "MMD0"),
    ("ANDR.MMD", "MMD1"), ("BAKE.MMD", "MMD1"), ("CENT.MMD", "MMD1"), ("COCA.MMD", "MMD1"), ("DIGI.MMD", "MMD1"), ("DORI.MMD", "MMD1"), ("ETEM.MMD", "MMD1"), ("GIRO.MMD", "MMD1"), ("HOUO.MMD", "MMD1"), ("IGAM.MMD", "MMD1"), ("KUNE.MMD", "MMD1"), ("KUWA.MMD", "MMD1"), ("LEOM.MMD", "MMD1"), ("MGDR.MMD", "MMD1"), ("MOJA.MMD", "MMD1"), ("MONO.MMD", "MMD1"), ("NANI.MMD", "MMD1"), ("OGRE.MMD", "MMD1"), ("PALM.MMD", "MMD1"), ("PATA.MMD", "MMD1"), ("PENM.MMD", "MMD1"), ("PICC.MMD", "MMD1"), ("PIYO.MMD", "MMD1"), ("SCUM.MMD", "MMD1"), ("SHEL.MMD", "MMD1"), ("SIRA.MMD", "MMD1"), ("TANE.MMD", "MMD1"), ("TOKO.MMD", "MMD1"), ("UNIM.MMD", "MMD1"), ("YURA.MMD", "MMD1"),
    ("ARUR.MMD", "MMD2"), ("DOKU.MMD", "MMD2"), ("FLAR.MMD", "MMD2"), ("FUGA.MMD", "MMD2"), ("GERE.MMD", "MMD2"), ("GGDR.MMD", "MMD2"), ("GOBR.MMD", "MMD2"), ("GOTU.MMD", "MMD2"), ("HKAB.MMD", "MMD2"), ("HYOG.MMD", "MMD2"), ("JMOJ.MMD", "MMD2"), ("MGSD.MMD", "MMD2"), ("MODO.MMD", "MMD2"), ("MTET.MMD", "MMD2"), ("NISE.MMD", "MMD2"), ("PANJ.MMD", "MMD2"), ("PCSC.MMD", "MMD2"), ("PIDD.MMD", "MMD2"), ("PSYC.MMD", "MMD2"), ("REDV.MMD", "MMD2"), ("SIMA.MMD", "MMD2"), ("TANK.MMD", "MMD2"), ("TOYA.MMD", "MMD2"), ("TUTI.MMD", "MMD2"), ("VAND.MMD", "MMD2"), ("VARM.MMD", "MMD2"), ("WARU.MMD", "MMD2"), ("YANM.MMD", "MMD2"), ("YKAG.MMD", "MMD2"),
    ("AKAT.MMD", "MMD3"), ("ANLG.MMD", "MMD3"), ("BLMR.MMD", "MMD3"), ("CLEA.MMD", "MMD3"), ("DKRZ.MMD", "MMD3"), ("GARD.MMD", "MMD3"), ("GECO.MMD", "MMD3"), ("GOLE.MMD", "MMD3"), ("GRUR.MMD", "MMD3"), ("ICDV.MMD", "MMD3"), ("ICEM.MMD", "MMD3"), ("INSE.MMD", "MMD3"), ("JIJI.MMD", "MMD3"), ("MCHO.MMD", "MMD3"), ("MRIS.MMD", "MMD3"), ("MUGE.MMD", "MMD3"), ("OTAM.MMD", "MMD3"), ("SABD.MMD", "MMD3"), ("SHAM.MMD", "MMD3"), ("SNDY.MMD", "MMD3"), ("SNGB.MMD", "MMD3"), ("SOUL.MMD", "MMD3"), ("TENS.MMD", "MMD3"), ("TENT.MMD", "MMD3"), ("TKKA.MMD", "MMD3"), ("TONO.MMD", "MMD3"), ("TUKA.MMD", "MMD3"), ("WRSE.MMD", "MMD3"), ("ZASS.MMD", "MMD3"),
    ("BRAK.MMD", "MMD4"), ("BRIK.MMD", "MMD4"), ("EAIR.MMD", "MMD4"), ("EANG.MMD", "MMD4"), ("EBET.MMD", "MMD4"), ("EBIR.MMD", "MMD4"), ("EDEV.MMD", "MMD4"), ("EELE.MMD", "MMD4"), ("EGAB.MMD", "MMD4"), ("EGAR.MMD", "MMD4"), ("EGOB.MMD", "MMD4"), ("EGRE.MMD", "MMD4"), ("EHOE.MMD", "MMD4"), ("EKAB.MMD", "MMD4"), ("EMAM.MMD", "MMD4"), ("EMER.MMD", "MMD4"), ("EMON.MMD", "MMD4"), ("EMTG.MMD", "MMD4"), ("EMTM.MMD", "MMD4"), ("ENUM.MMD", "MMD4"), ("ESEA.MMD", "MMD4"), ("ESKU.MMD", "MMD4"), ("ETYR.MMD", "MMD4"), ("EVEG.MMD", "MMD4"), ("EYUK.MMD", "MMD4"), ("HAGU.MMD", "MMD4"), ("JURE.MMD", "MMD4"), ("PUTI.MMD", "MMD4"), ("SCUD.MMD", "MMD4"), ("TIRS.MMD", "MMD4"),
    ("CEGR.MMD", "MMD5"), ("CEMG.MMD", "MMD5"), ("EAND.MMD", "MMD5"), ("EBAK.MMD", "MMD5"), ("ECEN.MMD", "MMD5"), ("ECOC.MMD", "MMD5"), ("EDIG.MMD", "MMD5"), ("EDOR.MMD", "MMD5"), ("EETE.MMD", "MMD5"), ("EGIR.MMD", "MMD5"), ("EIGA.MMD", "MMD5"), ("EKUN.MMD", "MMD5"), ("EKUW.MMD", "MMD5"), ("ELEO.MMD", "MMD5"), ("EMGD.MMD", "MMD5"), ("EMNO.MMD", "MMD5"), ("EMOJ.MMD", "MMD5"), ("ENAN.MMD", "MMD5"), ("EOGR.MMD", "MMD5"), ("EPAL.MMD", "MMD5"), ("EPAT.MMD", "MMD5"), ("EPEN.MMD", "MMD5"), ("EPIC.MMD", "MMD5"), ("EPIY.MMD", "MMD5"), ("ESCU.MMD", "MMD5"), ("ESHE.MMD", "MMD5"), ("ESIR.MMD", "MMD5"), ("EUNI.MMD", "MMD5"), ("EVAN.MMD", "MMD5"), ("EVED.MMD", "MMD5"),
];

fn build_chdat_mapping() -> HashMap<String, String> {
    CHDAT_MMD_MAPPING.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

pub struct Extractor {
    file_table: FileTable,
    output_dir: PathBuf,
    pub config: ExtractionConfig,
    pub force_overwrite: bool,
    pub verify_formats: bool,
    pub preserve_chdat_structure: bool,
    chdat_mapping: HashMap<String, String>,
    /// Reference hashes used to check the extracted files. Without a list every file is
    /// reported as `NoReference`.
    redump: Option<RedumpDatabase>,
}

impl Extractor {
    pub fn new(output_dir: PathBuf, force_overwrite: bool, verify_formats: bool) -> Result<Self> {
        Self::with_config(output_dir, force_overwrite, verify_formats, ExtractionConfig::default(), false)
    }
    
    pub fn new_with_chdat(output_dir: PathBuf, force_overwrite: bool, verify_formats: bool, preserve_chdat_structure: bool) -> Result<Self> {
        Self::with_config(output_dir, force_overwrite, verify_formats, ExtractionConfig::default(), preserve_chdat_structure)
    }

    pub fn with_config(
        output_dir: PathBuf,
        force_overwrite: bool,
        verify_formats: bool,
        config: ExtractionConfig,
        preserve_chdat_structure: bool,
    ) -> Result<Self> {
        let file_table = FileTable::load_us()?;

        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)?;
        }

        Ok(Self {
            file_table,
            output_dir,
            config,
            force_overwrite,
            verify_formats,
            preserve_chdat_structure,
            chdat_mapping: build_chdat_mapping(),
            redump: None,
        })
    }

    /// Uses the given reference list to check the extracted files.
    pub fn with_redump(mut self, redump: RedumpDatabase) -> Self {
        self.redump = Some(redump);
        self
    }

    /// True when a reference list is available to check the extracted files.
    pub fn has_reference(&self) -> bool {
        self.redump.as_ref().map(|db| !db.is_empty()).unwrap_or(false)
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

    fn sanitize_filename(&self, filename: &str) -> String {
        filename.replace('\\', "_")
    }

    fn get_chdat_subdir(&self, filename: &str) -> PathBuf {
        let upper = filename.to_uppercase();
        
        if upper == "ALLTIM.TIM" {
            return PathBuf::new();
        }
        
        if upper.ends_with(".MMD") {
            if let Some(subdir) = self.chdat_mapping.get(&upper) {
                return PathBuf::from(subdir);
            }
        }
        
        self.config.get_output_subdir(filename)
    }

    pub fn extract_file(&self, bin_reader: &mut crate::bin_parser::BinReader, entry: &FileEntry) -> Result<ExtractionFileResult> {
        let sanitized_filename = self.sanitize_filename(&entry.filename);
        
        let output_subdir = if self.preserve_chdat_structure {
            self.get_chdat_subdir(&entry.filename)
        } else {
            self.config.get_output_subdir(&entry.filename)
        };
        
        let output_path = self.output_dir.join(output_subdir).join(&sanitized_filename);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let is_str = entry.filename.to_ascii_uppercase().ends_with(".STR");
        let data = if is_str {
            // The US table counts STR data at 2336 bytes per sector; convert-str
            // consumes the complete raw 2352-byte sectors.
            const TABLE_STR_SECTOR_SIZE: u32 = 2336;
            let sector_count = entry.size.checked_add(TABLE_STR_SECTOR_SIZE - 1)
                .ok_or_else(|| DwExtractError::InvalidBin("STR size overflows sector count".to_string()))?
                / TABLE_STR_SECTOR_SIZE;
            bin_reader.read_raw_sectors(entry.sector, sector_count)?
        } else {
            bin_reader.read_file_data(entry.sector, entry.size)?
        };
        let extracted_size = u32::try_from(data.len())
            .map_err(|_| DwExtractError::InvalidBin("Extracted file exceeds 4 GiB".to_string()))?;

        let hashes = compute_hashes(&data);

        let should_write = if output_path.exists() {
            if self.force_overwrite {
                true
            } else {
                let existing_data = fs::read(&output_path)?;
                let existing_hashes = compute_hashes(&existing_data);
                if existing_hashes.sha256 == hashes.sha256 {
                    println!("  {} (unchanged, skipping)", entry.filename);
                    let verification_status =
                        self.verification_status(&entry.filename, data.len() as u32, &hashes);
                    return Ok(ExtractionFileResult {
                        path: entry.filename.clone(),
                        size: extracted_size,
                        sha256: hashes.sha256,
                        md5: hashes.md5,
                        verification_status,
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
            println!("  Extracted: {} ({} bytes)", entry.filename, extracted_size);
        }

        let format_valid = if self.verify_formats {
            self.verify_format(&entry.filename, &data)?
        } else {
            true
        };

        let verification_status = self.verification_status(&entry.filename, data.len() as u32, &hashes);

        Ok(ExtractionFileResult {
            path: entry.filename.clone(),
            size: extracted_size,
            sha256: hashes.sha256,
            md5: hashes.md5,
            verification_status,
            format_valid,
            error: None,
        })
    }

    /// The status of a file against the reference list: `NoReference` when there is no
    /// list or the list does not mention the file.
    fn verification_status(&self, filename: &str, size: u32, hashes: &crate::verification::FileHashes) -> VerificationStatus {
        match &self.redump {
            Some(database) => database.verify(filename, size, hashes),
            None => VerificationStatus::NoReference,
        }
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
        assert_eq!(extractor.list_files().len(), 818);
        assert_eq!(extractor.list_mmd_files().len(), 178);
        assert_eq!(extractor.list_tim_files().len(), 52);
    }

    #[test]
    fn test_extractor_with_config() {
        let dir = tempdir().unwrap();
        let config = ExtractionConfig::load_default().unwrap();
        let extractor = Extractor::with_config(dir.path().to_path_buf(), false, false, config, false).unwrap();
        assert_eq!(extractor.list_files().len(), 818);
    }
}
