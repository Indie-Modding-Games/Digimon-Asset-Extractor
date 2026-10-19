use crate::error::Result;
use crate::verification::Manifest;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "dw-extract")]
#[command(about = "Digimon World (SLUS-01032) Asset Extractor", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Extract all files from BIN/CUE
    Extract {
        /// Path to .bin file
        #[arg(short, long)]
        bin: PathBuf,
        
        /// Path to .cue file
        #[arg(short, long)]
        cue: PathBuf,
        
        /// Output directory
        #[arg(short, long, default_value = "assets")]
        output: PathBuf,
        
        /// Force overwrite existing files
        #[arg(long)]
        force: bool,
        
        /// Skip format validation
        #[arg(long)]
        no_verify: bool,
        
        /// Generate manifest JSON
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    
    /// Extract specific files by name
    ExtractFiles {
        /// Path to .bin file
        #[arg(short, long)]
        bin: PathBuf,
        
        /// Path to .cue file
        #[arg(short, long)]
        cue: PathBuf,
        
        /// Output directory
        #[arg(short, long, default_value = "assets")]
        output: PathBuf,
        
        /// File names to extract (e.g., ALLTIM.TIM AGUM.MMD)
        #[arg(required = true)]
        files: Vec<String>,
        
        /// Force overwrite existing files
        #[arg(long)]
        force: bool,
        
        /// Skip format validation
        #[arg(long)]
        no_verify: bool,
        
        /// Generate manifest JSON
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    
    /// Convert TIM files to PNG
    ConvertTim {
        /// Input TIM file or directory
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for PNG files
        #[arg(short, long)]
        output: PathBuf,
        
        /// Process all TIM files in directory recursively
        #[arg(long)]
        recursive: bool,
    },
    
    /// Extract raw VAB blocks from VHB files
    ConvertAudio {
        /// Input VHB file or directory
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for raw VAB blocks
        #[arg(short, long)]
        output: PathBuf,
    },
    
    /// List all files in the file table
    List {
        /// Show only MMD files
        #[arg(long)]
        mmd: bool,
        
        /// Show only TIM files
        #[arg(long)]
        tim: bool,
    },
    
    /// Verify existing extracted assets against manifest
    Verify {
        /// Path to manifest JSON
        #[arg(short, long)]
        manifest: PathBuf,
        
        /// Assets directory
        #[arg(short, long, default_value = "assets")]
        assets: PathBuf,
    },
    
    /// Show file table info
    Info,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Extract { bin, cue, output, force, no_verify, manifest } => {
            run_extract(&bin, &cue, &output, force, !no_verify, manifest.as_deref())
        }
        Commands::ExtractFiles { bin, cue, output, files, force, no_verify, manifest } => {
            run_extract_files(&bin, &cue, &output, &files, force, manifest.as_deref())
        }
        Commands::ConvertTim { input, output, recursive } => {
            run_convert_tim(&input, &output, recursive)
        }
        Commands::ConvertAudio { input, output } => {
            run_convert_audio(&input, &output)
        }
        Commands::List { mmd, tim } => {
            run_list(mmd, tim)
        }
        Commands::Verify { manifest, assets } => {
            run_verify(&manifest, &assets)
        }
        Commands::Info => {
            run_info()
        }
    }
}

fn run_extract(bin_path: &PathBuf, cue_path: &PathBuf, output: &PathBuf, force: bool, verify: bool, manifest_path: Option<&Path>) -> Result<()> {
    let cue_info = crate::bin_parser::parse_cue(cue_path)?;
    
    let bin_full_path = cue_path.parent().unwrap_or(std::path::Path::new("")).join(&cue_info.bin_path);
    let bin_path = if bin_path.exists() { bin_path } else { &bin_full_path };
    
    let mut bin_reader = crate::bin_parser::BinReader::open(bin_path)?;
    
    let extractor = crate::extractor::Extractor::new(output.to_path_buf(), force, false)?;
    
    let result = extractor.extract_all(&mut bin_reader)?;
    
    println!("Extracted {} files to {}", result.total_files, output.display());
    
    if let Some(manifest_path) = manifest_path {
        let manifest = Manifest::from_extraction_result(&result, &bin_path.display().to_string(), &cue_path.display().to_string());
        manifest.save(manifest_path)?;
    }
    
    if result.errors > 0 {
        std::process::exit(2);
    }
    
    Ok(())
}

fn run_extract_files(bin_path: &PathBuf, cue_path: &PathBuf, output: &PathBuf, files: &[String], force: bool, manifest_path: Option<&Path>) -> Result<()> {
    let cue_info = crate::bin_parser::parse_cue(cue_path)?;
    
    let bin_full_path = cue_path.parent().unwrap_or(std::path::Path::new("")).join(&cue_info.bin_path);
    let bin_path = if bin_path.exists() { bin_path } else { &bin_full_path };
    
    let mut bin_reader = crate::bin_parser::BinReader::open(bin_path)?;
    
    let extractor = crate::extractor::Extractor::new(output.to_path_buf(), force, false)?;
    
    let result = extractor.extract_selected(&mut bin_reader, files)?;
    
    println!("Extracted {} files to {}", result.total_files, output.display());
    
    if let Some(manifest_path) = manifest_path {
        let manifest = Manifest::from_extraction_result(&result, &bin_path.display().to_string(), &cue_path.display().to_string());
        manifest.save(manifest_path)?;
    }
    
    if result.errors > 0 {
        std::process::exit(2);
    }
    
    Ok(())
}

fn run_convert_tim(input: &PathBuf, output: &PathBuf, _recursive: bool) -> Result<()> {
    use walkdir::WalkDir;
    
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }
    
    let is_input_dir = input.is_dir();
    
    if !is_input_dir {
        // Single file
        std::fs::create_dir_all(output)?;
        let data = std::fs::read(input)?;
        
        match crate::formats::tim::TimHeader::to_png(&data, output) {
            Ok(_) => println!("Converted TIM to PNG: {}", output.display()),
            Err(e) => eprintln!("Failed to convert TIM: {}", e),
        }
    } else {
        // Directory - process all TIM files recursively
        std::fs::create_dir_all(output)?;
        
        let mut converted = 0;
        let mut failed = 0;
        
        for entry in WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_uppercase();
                if ext == "TIM" {
                    let data = std::fs::read(path)?;
                    let out_path = output.join(path.file_name().unwrap()).with_extension("png");
                    
                    match crate::formats::tim::TimHeader::to_png(&data, &out_path) {
                        Ok(_) => { converted += 1; }
                        Err(_) => { failed += 1; }
                    }
                }
            }
        }
        
        println!("TIM conversion: {} converted, {} failed", converted, failed);
    }
    
    Ok(())
}

fn run_convert_audio(input: &PathBuf, output: &PathBuf) -> Result<()> {
    use walkdir::WalkDir;
    
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }
    
    let is_input_dir = input.is_dir();
    
    if !is_input_dir {
        // Single file
        std::fs::create_dir_all(output)?;
        let data = std::fs::read(input)?;
        
        // Try VAB first (for .VHB files which are actually VAB format)
        // Use parse_all to handle multiple VABs in one file
        let vabs = crate::formats::vab::VabFile::parse_all(&data)?;
        if !vabs.is_empty() {
            for (i, vab) in vabs.iter().enumerate() {
                // Save raw VAB block for external tool processing
                let raw_path = output.join(format!("vab_{:03}.raw", i));
                std::fs::write(&raw_path, &vab.vag_data)?;
            }
            println!("Extracted {} raw VAB blocks to {}", vabs.len(), output.display());
        } else if let Ok(vhb) = crate::formats::vag::VhbFile::parse(&data) {
            let files = vhb.to_wav_files(output)?;
            println!("Converted {} VAG streams to WAV", files.len());
        } else {
            return Err(crate::error::DwExtractError::InvalidBin("Not a valid VAB or VHB file".to_string()));
        }
    } else {
        // Directory - process all VHB/VAB files recursively
        std::fs::create_dir_all(output)?;
        
        let mut total_streams = 0;
        let mut total_files = 0;
        
        for entry in WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_uppercase();
                if ext == "VHB" {
                    total_files += 1;
                    let data = std::fs::read(path)?;
                    
                    // Try VAB first
                    let vabs = crate::formats::vab::VabFile::parse_all(&data)?;
                    if !vabs.is_empty() {
                        let stem = path.file_stem().unwrap().to_string_lossy();
                        for (i, vab) in vabs.iter().enumerate() {
                            let raw_path = output.join(format!("{}_vab_{:03}.raw", stem, i));
                            std::fs::write(&raw_path, &vab.vag_data)?;
                            total_streams += 1;
                        }
                    } else if let Ok(vhb) = crate::formats::vag::VhbFile::parse(&data) {
                        let files = vhb.to_wav_files(output)?;
                        total_streams += files.len();
                    }
                }
            }
        }
        
        println!("Extracted {} raw VAB blocks from {} VHB files", total_streams, total_files);
    }
    
    Ok(())
}

fn run_list(mmd: bool, tim: bool) -> Result<()> {
    let extractor = crate::extractor::Extractor::new(std::path::PathBuf::from("."), false, false)?;
    
    let files = if mmd {
        extractor.list_mmd_files()
    } else if tim {
        extractor.list_tim_files()
    } else {
        extractor.list_files()
    };
    
    println!("{:<40} {:>10} {:>10}", "FILENAME", "SECTOR", "SIZE");
    println!("{}", "-".repeat(62));
    
    for entry in &files {
        println!("{:<40} {:>10} {:>10}", entry.filename, entry.sector, entry.size);
    }
    
    println!("\nTotal: {} files", files.len());
    
    Ok(())
}

fn run_verify(manifest_path: &PathBuf, assets_path: &PathBuf) -> Result<()> {
    let manifest_content = std::fs::read_to_string(manifest_path)?;
    let manifest: Manifest = serde_json::from_str(&manifest_content)?;
    
    println!("Verifying against manifest: {}", manifest_path.display());
    println!("Assets directory: {}", assets_path.display());
    println!("Version: {}", manifest.version);
    
    let mut verified = 0;
    let mut mismatched = 0;
    let mut missing = 0;
    
    for file in &manifest.files {
        let asset_path = assets_path.join(&file.path);
        
        if !asset_path.exists() {
            println!("  MISSING: {}", file.path);
            missing += 1;
            continue;
        }
        
        let hashes = crate::verification::compute_file_hashes(&asset_path)?;
        
        if hashes.sha256 == file.sha256 && hashes.md5 == file.md5 {
            println!("  OK: {}", file.path);
            verified += 1;
        } else {
            println!("  MISMATCH: {} (expected SHA256: {}, got: {})", file.path, file.sha256, hashes.sha256);
            mismatched += 1;
        }
    }
    
    println!("\n=== Verification Summary ===");
    println!("Verified: {}", verified);
    println!("Mismatched: {}", mismatched);
    println!("Missing: {}", missing);
    
    if mismatched > 0 || missing > 0 {
        std::process::exit(3);
    }
    
    Ok(())
}

fn run_info() -> Result<()> {
    let extractor = crate::extractor::Extractor::new(std::path::PathBuf::from("."), false, false)?;
    
    println!("Digimon World Asset Extractor - File Table Info");
    println!("==============================================");
    println!("Version: SLUS-01032 (US)");
    println!("Source: dw_decomp (https://github.com/jype0/dw_decomp)");
    println!();
    println!("Total files in table: {}", extractor.list_files().len());
    println!("MMD files: {}", extractor.list_mmd_files().len());
    println!("TIM files: {}", extractor.list_tim_files().len());
    println!();
    println!("Key files:");
    println!("  SYSTEM.CNF           - Version identification");
    println!("  CHDAT/ALLTIM.TIM     - Main texture archive (~3.2 MB)");
    println!("  *.MMD                - Digimon model files (178 files)");
    
    Ok(())
}