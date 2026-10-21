use crate::bin_parser::{BinReader, parse_cue};
use crate::config::ExtractionConfig;
use crate::extractor::Extractor;
use crate::file_table::FileTable;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

#[derive(Debug, Clone)]
pub enum ExtractionMsg {
    Started,
    Progress { current: usize, total: usize, current_file: String },
    FileExtracted { name: String, size: u32 },
    Verification { verified: usize, mismatched: usize, errors: usize },
    Finished { success: bool, message: String, output_dir: PathBuf },
    Error(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    pub cue_path: PathBuf,
    pub bin_path: PathBuf,
    pub output_dir: PathBuf,
    pub extract_all: bool,
    pub force_overwrite: bool,
    pub verify_formats: bool,
    pub preserve_chdat_structure: bool,
    pub config: Option<ExtractionConfig>,
}

pub fn run_extraction(
    options: ExtractionOptions, 
    tx: mpsc::Sender<ExtractionMsg>,
    cancelled: Arc<AtomicBool>
) {
    thread::spawn(move || {
        let result = do_extract(options, tx.clone(), cancelled);
        
        match result {
            Ok((output_dir, total, verified, mismatched, errors)) => {
                let _ = tx.send(ExtractionMsg::Finished {
                    success: errors == 0,
                    message: format!(
                        "Extracción completada: {} archivos ({} verificados, {} discrepancias, {} errores)",
                        total, verified, mismatched, errors
                    ),
                    output_dir,
                });
            }
            Err(e) => {
                let _ = tx.send(ExtractionMsg::Error(e.to_string()));
            }
        }
    });
}

fn do_extract(
    options: ExtractionOptions, 
    tx: mpsc::Sender<ExtractionMsg>,
    cancelled: Arc<AtomicBool>
) -> Result<(PathBuf, usize, usize, usize, usize), Box<dyn std::error::Error + Send + Sync>> {
    let cue_info = parse_cue(&options.cue_path)?;
    
    let bin_full_path = options.cue_path.parent()
        .unwrap_or(std::path::Path::new(""))
        .join(&cue_info.bin_path);
    let bin_path = if options.bin_path.exists() { 
        &options.bin_path 
    } else { 
        &bin_full_path 
    };

    let mut bin_reader = BinReader::open(bin_path)?;
    
    let file_table = FileTable::load_us()?;
    let _version = file_table.verify_version(&mut bin_reader)?;
    
    let _ = tx.send(ExtractionMsg::Started);
    
    let extractor = if options.preserve_chdat_structure {
        if let Some(config) = options.config {
            Extractor::with_config(
                options.output_dir.clone(),
                options.force_overwrite,
                options.verify_formats,
                config,
                true,
            )?
        } else {
            let default_config = ExtractionConfig::default();
            Extractor::with_config(
                options.output_dir.clone(),
                options.force_overwrite,
                options.verify_formats,
                default_config,
                true,
            )?
        }
    } else if let Some(config) = options.config {
        Extractor::with_config(
            options.output_dir.clone(),
            options.force_overwrite,
            options.verify_formats,
            config,
            false,
        )?
    } else {
        Extractor::new(
            options.output_dir.clone(),
            options.force_overwrite,
            options.verify_formats,
        )?
    };

    let files_to_extract: Vec<String> = if options.extract_all {
        file_table.all_entries().map(|e| e.filename.clone()).collect()
    } else {
        vec!["SYSTEM.CNF".to_string(), "ALLTIM.TIM".to_string()]
    };

    let total = files_to_extract.len();
    let mut verified = 0;
    let mut mismatched = 0;
    let mut errors = 0;

    for (i, name) in files_to_extract.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            let _ = tx.send(ExtractionMsg::Cancelled);
            return Err("Extracción cancelada por el usuario".into());
        }

        let entry = file_table.get(name)
            .ok_or_else(|| format!("Archivo no encontrado en tabla: {}", name))?;

        let _ = tx.send(ExtractionMsg::Progress {
            current: i + 1,
            total,
            current_file: name.clone(),
        });

        match extractor.extract_file(&mut bin_reader, entry) {
            Ok(result) => {
                let _ = tx.send(ExtractionMsg::FileExtracted { 
                    name: name.clone(), 
                    size: entry.size 
                });
                match result.verification_status {
                    crate::verification::VerificationStatus::Verified => verified += 1,
                    crate::verification::VerificationStatus::Mismatch => mismatched += 1,
                    crate::verification::VerificationStatus::NoReference => {}
                }
            }
            Err(e) => {
                errors += 1;
                let _ = tx.send(ExtractionMsg::Error(format!("Error extrayendo {}: {}", name, e)));
            }
        }
    }

    Ok((options.output_dir, total, verified, mismatched, errors))
}