use crate::config::ExtractionConfig;
use crate::error::Result;
use crate::verification::Manifest;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

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

        /// Reference hash list (Redump format) used to verify the extracted files
        #[arg(long)]
        redump: Option<PathBuf>,

        /// Use structured output folders (YAML config)
        #[arg(long)]
        structured: bool,

        /// Path to custom YAML structure config
        #[arg(long)]
        structure_config: Option<PathBuf>,

        /// Force flat output (ignore structure config)
        #[arg(long)]
        flat: bool,

        /// Preserve original CHDAT folder structure (MMD0-MMD5 subfolders)
        #[arg(long)]
        preserve_chdat: bool,
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

        /// Reference hash list (Redump format) used to verify the extracted files
        #[arg(long)]
        redump: Option<PathBuf>,

        /// Use structured output folders (YAML config)
        #[arg(long)]
        structured: bool,

        /// Path to custom YAML structure config
        #[arg(long)]
        structure_config: Option<PathBuf>,

        /// Force flat output (ignore structure config)
        #[arg(long)]
        flat: bool,

        /// Preserve original CHDAT folder structure (MMD0-MMD5 subfolders)
        #[arg(long)]
        preserve_chdat: bool,
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
    
    /// Convert VHB/VAB audio files to WAV, SF2 or MIDI
    ConvertAudio {
        /// Input VHB/VAB/SEQ file or directory
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for the audio files
        #[arg(short, long)]
        output: PathBuf,
        
        /// Output format
        #[arg(long, default_value = "wav", value_parser = ["wav", "raw", "sf2", "midi"])]
        format: String,
        
        /// Sample rate (default: 37800 for VAG)
        #[arg(long, default_value = "37800")]
        sample_rate: u32,
        
        /// Extract all VAGs as separate files
        #[arg(long)]
        extract_vags: bool,
        
        /// Process directory recursively
        #[arg(long)]
        recursive: bool,
    },
    
    /// Convert EFE effect files to YAML
    ConvertEfe {
        /// Input EFE file (e.g., EFEDAT.EFE)
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for YAML and assets
        #[arg(short, long)]
        output: PathBuf,
        
        /// Extract TMD/TIM assets
        #[arg(long)]
        extract_assets: bool,
        
        /// Output format
        #[arg(long, default_value = "yaml", value_parser = ["yaml", "json"])]
        format: String,
    },
    
    /// Convert MAP/TFS map files to YAML/OBJ
    ConvertMap {
        /// Input MAP file
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for YAML/OBJ and assets
        #[arg(short, long)]
        output: PathBuf,
        
        /// Extract TIM textures from MAP
        #[arg(long)]
        extract_textures: bool,
        
        /// Output format
        #[arg(long, default_value = "yaml", value_parser = ["yaml", "obj", "both"])]
        format: String,
        
        /// Number of 8-bit images (auto-detected if not specified)
        #[arg(long)]
        images_8: Option<i32>,
        
        /// Number of 4-bit images (auto-detected if not specified)
        #[arg(long)]
        images_4: Option<i32>,
        
        /// Map has NPCs (auto-detected if not specified)
        #[arg(long)]
        npcs: Option<bool>,
    },
    
    /// Convert STR video files to frames and audio
    ConvertStr {
        /// Input STR file
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for frames and audio
        #[arg(short, long)]
        output: PathBuf,
        
        /// Extract video frames (MDEC)
        #[arg(long)]
        extract_video: bool,
        
        /// Extract audio tracks (XA-ADPCM to WAV)
        #[arg(long)]
        extract_audio: bool,
    },
    
    /// Convert SCN script files to YAML/disassembly
    ConvertScn {
        /// Input SCN file (e.g., DG.SCN, MAPHEAD.SCN)
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for YAML and disassembly
        #[arg(short, long)]
        output: PathBuf,
        
        /// Output format
        #[arg(long, default_value = "yaml", value_parser = ["yaml", "disasm"])]
        format: String,
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
        Commands::Extract { bin, cue, output, force, no_verify, manifest, redump, structured, structure_config, flat, preserve_chdat } => {
            run_extract(&bin, &cue, &output, force, !no_verify, manifest.as_deref(), redump.as_deref(), structured, structure_config.as_deref(), flat, preserve_chdat)
        }
        Commands::ExtractFiles { bin, cue, output, files, force, no_verify, manifest, redump, structured, structure_config, flat, preserve_chdat } => {
            run_extract_files(&bin, &cue, &output, &files, force, !no_verify, manifest.as_deref(), redump.as_deref(), structured, structure_config.as_deref(), flat, preserve_chdat)
        }
        Commands::ConvertTim { input, output, recursive } => {
            run_convert_tim(&input, &output, recursive)
        }
        Commands::ConvertAudio { input, output, format, sample_rate, extract_vags, recursive } => {
            run_convert_audio(&input, &output, &format, sample_rate, extract_vags, recursive)
        }
        Commands::ConvertEfe { input, output, extract_assets, format } => {
            run_convert_efe(&input, &output, extract_assets, &format)
        }
        Commands::ConvertMap { input, output, extract_textures, format, images_8, images_4, npcs } => {
            run_convert_map(&input, &output, extract_textures, &format, images_8, images_4, npcs)
        }
        Commands::ConvertStr { input, output, extract_video, extract_audio } => {
            run_convert_str(&input, &output, extract_video, extract_audio)
        }
        Commands::ConvertScn { input, output, format } => {
            run_convert_scn(&input, &output, &format)
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

fn load_structure_config(structured: bool, structure_config: Option<&Path>, flat: bool, preserve_chdat: bool) -> Option<ExtractionConfig> {
    if flat || (!structured && !preserve_chdat) {
        return None;
    }
    
    if let Some(path) = structure_config {
        Some(ExtractionConfig::load_from_file(path).expect("Failed to load structure config"))
    } else {
        Some(ExtractionConfig::structured())
    }
}

/// Builds the extractor used by the extraction commands.
///
/// The `verify_formats` flag is the one controlled by `--no-verify` and the reference list
/// is optional: without it every file is reported as `NoReference`.
fn build_extractor(
    output: &Path,
    force: bool,
    verify_formats: bool,
    config: Option<ExtractionConfig>,
    preserve_chdat: bool,
    redump: Option<&Path>,
) -> Result<crate::extractor::Extractor> {
    let mut extractor = if let Some(config) = config {
        crate::extractor::Extractor::with_config(output.to_path_buf(), force, verify_formats, config, preserve_chdat)?
    } else if preserve_chdat {
        crate::extractor::Extractor::new_with_chdat(output.to_path_buf(), force, verify_formats, true)?
    } else {
        crate::extractor::Extractor::new(output.to_path_buf(), force, verify_formats)?
    };

    if let Some(path) = redump {
        let database = crate::verification::RedumpDatabase::load_from_file(path)?;
        println!("Reference list: {} entries from {}", database.len(), path.display());
        extractor = extractor.with_redump(database);
    }

    Ok(extractor)
}

fn run_extract(bin_path: &PathBuf, cue_path: &PathBuf, output: &PathBuf, force: bool, verify: bool, manifest_path: Option<&Path>, redump: Option<&Path>, structured: bool, structure_config: Option<&Path>, flat: bool, preserve_chdat: bool) -> Result<()> {
    let cue_info = crate::bin_parser::parse_cue(cue_path)?;
    
    let bin_full_path = cue_path.parent().unwrap_or(std::path::Path::new("")).join(&cue_info.bin_path);
    let bin_path = if bin_path.exists() { bin_path } else { &bin_full_path };
    
    let mut bin_reader = crate::bin_parser::BinReader::open(bin_path)?;
    
    let config = load_structure_config(structured, structure_config, flat, preserve_chdat);
    let extractor = build_extractor(output, force, verify, config, preserve_chdat, redump)?;
    
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

fn run_extract_files(bin_path: &PathBuf, cue_path: &PathBuf, output: &PathBuf, files: &[String], force: bool, verify: bool, manifest_path: Option<&Path>, redump: Option<&Path>, structured: bool, structure_config: Option<&Path>, flat: bool, preserve_chdat: bool) -> Result<()> {
    let cue_info = crate::bin_parser::parse_cue(cue_path)?;
    
    let bin_full_path = cue_path.parent().unwrap_or(std::path::Path::new("")).join(&cue_info.bin_path);
    let bin_path = if bin_path.exists() { bin_path } else { &bin_full_path };
    
    let mut bin_reader = crate::bin_parser::BinReader::open(bin_path)?;
    
    let config = load_structure_config(structured, structure_config, flat, preserve_chdat);
    let extractor = build_extractor(output, force, verify, config, preserve_chdat, redump)?;
    
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

/// The TIM files to convert: the file itself, or the files of a directory.
/// Without `--recursive` only the files of the first level are used.
fn collect_tim_files(input: &Path, recursive: bool) -> Vec<std::path::PathBuf> {
    use walkdir::WalkDir;

    if !input.is_dir() {
        return vec![input.to_path_buf()];
    }

    let depth = if recursive { usize::MAX } else { 1 };

    WalkDir::new(input)
        .max_depth(depth)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("TIM"))
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

/// The path of the PNG of a TIM file inside the output directory, without repeating names.
/// A TIM with several palettes writes one file per palette, so all of them are taken.
fn png_output_path(
    output: &Path,
    tim_path: &Path,
    used: &mut Vec<std::path::PathBuf>,
    palettes: usize,
) -> std::path::PathBuf {
    let stem = tim_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "tim".to_string());

    let taken = |candidate: &std::path::Path, used: &Vec<std::path::PathBuf>| {
        used.iter().any(|used_path| used_path.as_path() == candidate)
            || (0..palettes).any(|index| {
                used.iter().any(|used_path| {
                    *used_path == crate::formats::tim::palette_path(candidate, index)
                })
            })
    };

    let mut candidate = output.join(format!("{}.png", stem));
    let mut index = 1;
    while taken(&candidate, used) {
        candidate = output.join(format!("{}_{}.png", stem, index));
        index += 1;
    }

    used.push(candidate.clone());
    for palette in 0..palettes {
        used.push(crate::formats::tim::palette_path(&candidate, palette));
    }

    candidate
}

fn run_convert_tim(input: &PathBuf, output: &PathBuf, recursive: bool) -> Result<()> {
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }

    std::fs::create_dir_all(output)?;

    let tim_files = collect_tim_files(input, recursive);

    let mut converted = 0;
    let mut failed = 0;
    let mut used_names: Vec<std::path::PathBuf> = Vec::new();

    for tim_path in &tim_files {
        let data = match std::fs::read(tim_path) {
            Ok(data) => data,
            Err(e) => {
                failed += 1;
                eprintln!("Failed to read {}: {}", tim_path.display(), e);
                continue;
            }
        };

        let info = match crate::formats::tim::TimHeader::validate(&data) {
            Ok(info) => info,
            Err(e) => {
                failed += 1;
                eprintln!("Failed to convert {}: {}", tim_path.display(), e);
                continue;
            }
        };

        let out_path = png_output_path(output, tim_path, &mut used_names, info.palette_count);

        match crate::formats::tim::TimHeader::to_pngs(&data, &out_path) {
            Ok(files) => {
                converted += 1;
                for file in files {
                    println!("Converted TIM to PNG: {}", file.display());
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("Failed to convert {}: {}", tim_path.display(), e);
            }
        }
    }

    println!("TIM conversion: {} converted, {} failed", converted, failed);

    if failed > 0 {
        return Err(crate::error::DwExtractError::InvalidFormat(format!(
            "{} TIM file(s) could not be converted",
            failed
        )));
    }

    Ok(())
}

/// The samples of the VB of a sound bank, decoded, indexed by their number: the sample 1 is at the index 1.
fn decode_vag_samples(vhb: &crate::formats::vab2::Vhb) -> Result<Vec<crate::formats::vag::DecodedSample>> {
    let locations = crate::formats::vab2::locate_vags(&vhb.vab, vhb.vb_data.len())?;
    let count = locations.iter().map(|location| location.number).max().unwrap_or(0);
    let mut samples = vec![crate::formats::vag::DecodedSample::default(); count + 1];
    for location in &locations {
        let vag_data = &vhb.vb_data[location.offset..location.offset + location.size];
        let decoded = crate::formats::adpcm::decode_adpcm(vag_data);
        samples[location.number] = crate::formats::vag::DecodedSample {
            pcm: decoded.pcm,
            loop_info: decoded.loop_info,
        };
    }
    Ok(samples)
}

/// Converts one sound bank. Returns the number of extracted streams.
fn convert_audio_file(
    path: &Path,
    file_output: &Path,
    format: &str,
    sample_rate: u32,
    extract_vags: bool,
) -> Result<usize> {
    let data = std::fs::read(path)?;
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();

    if let Ok(vhb) = crate::formats::vab2::Vhb::from_bytes(&data) {
        println!("Parsed VHB: {} programs, {} tones, {} VAGs, music_font: {}",
            vhb.vab.header.ps, vhb.vab.header.ts, vhb.vab.header.vs, vhb.header.music_font);

        let mut streams = 0;

        if format == "raw" || extract_vags {
            let locations = crate::formats::vab2::locate_vags(&vhb.vab, vhb.vb_data.len())?;
            for loc in locations {
                let vag_data = &vhb.vb_data[loc.offset..loc.offset + loc.size];
                let out_name = if format == "raw" {
                    format!("vag_{:03}.raw", loc.number)
                } else {
                    format!("vag_{:03}.wav", loc.number)
                };
                let out_path = file_output.join(out_name);

                if format == "wav" {
                    crate::formats::wav::vag_to_wav(vag_data, &out_path, sample_rate)?;
                } else {
                    std::fs::write(&out_path, vag_data)?;
                }
                println!("  Extracted VAG {}: {} bytes", loc.number, loc.size);
                streams += 1;
            }
        }

        if format == "wav" && !extract_vags {
            let wav_path = file_output.join("output.wav");
            crate::formats::wav::vag_to_wav(&vhb.vb_data, &wav_path, sample_rate)?;
            println!("Converted VB to WAV: {}", wav_path.display());
        }

        if format == "sf2" {
            let sf2_path = file_output.join("output.sf2");
            let decoded_samples = decode_vag_samples(&vhb)?;

            let vab_sf2 = crate::formats::vab_sf2::convert_vab_to_soundfont(&vhb.vab, &decoded_samples, &stem)?;
            let sf2_data = crate::formats::sf2::write_sf2(&vab_sf2.font)?;
            std::fs::write(&sf2_path, sf2_data)?;
            println!("  Converted to SF2: {}", sf2_path.display());
        }

        println!("  {}: {} VAGs", stem, crate::formats::vab2::locate_vags(&vhb.vab, vhb.vb_data.len())?.len());

        return Ok(streams);
    }

    if let Ok(vhb) = crate::formats::vag::VhbFile::parse(&data) {
        let files = vhb.to_wav_files(file_output)?;
        println!("  {}: converted {} VAG streams to WAV (legacy parser)", stem, files.len());
        return Ok(files.len());
    }

    Err(crate::error::DwExtractError::InvalidBin(format!(
        "{} is not a valid VAB/VHB file",
        path.display()
    )))
}

fn run_convert_audio(
    input: &PathBuf, 
    output: &PathBuf, 
    format: &str, 
    sample_rate: u32, 
    extract_vags: bool,
    recursive: bool
) -> Result<()> {
    use walkdir::WalkDir;
    
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }
    
    let is_input_dir = input.is_dir();
    
    if format != "wav" && format != "raw" && format != "sf2" && format != "midi" {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Unsupported format: {}. Use wav, raw, sf2, or midi", format)));
    }
    
    if format == "midi" {
        return run_convert_audio_to_midi(input, output, recursive);
    }
    
    if !is_input_dir {
        // Single file
        std::fs::create_dir_all(output)?;
        convert_audio_file(input, output, format, sample_rate, extract_vags)?;
        return Ok(());
    }

    // Directory - process all VHB/VAB files
    std::fs::create_dir_all(output)?;

    let mut total_streams = 0;
    let mut total_files = 0;
    let mut failed_files = 0;

    let walker = if recursive {
        WalkDir::new(input).into_iter()
    } else {
        WalkDir::new(input).max_depth(1).into_iter()
    };
    
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_uppercase();
            if ext == "VHB" || ext == "VAB" {
                total_files += 1;
                let stem = path.file_stem().unwrap().to_string_lossy();
                let file_output = if total_files > 1 { output.join(&*stem) } else { output.to_path_buf() };
                std::fs::create_dir_all(&file_output)?;

                match convert_audio_file(path, &file_output, format, sample_rate, extract_vags) {
                    Ok(streams) => total_streams += streams,
                    Err(e) => {
                        failed_files += 1;
                        eprintln!("  Failed to convert {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    
    println!("Processed {} files, extracted {} VAG streams", total_files, total_streams);

    if failed_files > 0 {
        return Err(crate::error::DwExtractError::InvalidFormat(format!(
            "{} of {} sound file(s) could not be converted",
            failed_files,
            total_files
        )));
    }

    Ok(())
}

/// Writes a MIDI file of every sequence of a sound file: the tracks of a music font, or the sequence of a file.
fn run_convert_audio_to_midi(input: &PathBuf, output: &PathBuf, recursive: bool) -> Result<()> {
    use walkdir::WalkDir;

    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!(
            "Input path does not exist: {}",
            input.display()
        )));
    }

    std::fs::create_dir_all(output)?;

    let files: Vec<PathBuf> = if input.is_dir() {
        let walker = if recursive {
            WalkDir::new(input).into_iter()
        } else {
            WalkDir::new(input).max_depth(1).into_iter()
        };
        walker
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.into_path())
            .filter(|path| path.is_file())
            .collect()
    } else {
        vec![input.clone()]
    };

    // With more than one file, every one of them gets a directory of its own.
    let separate = files.len() > 1;
    let mut written = 0;
    for path in &files {
        let data = std::fs::read(path)?;
        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let dir = if separate {
            output.join(&stem)
        } else {
            output.to_path_buf()
        };
        std::fs::create_dir_all(&dir)?;

        // A plain sequence: one MIDI file with the name of the file.
        if data.starts_with(b"pQES") {
            let seq = crate::formats::audio::read_seq(&data)?;
            let ranges = crate::formats::audio::no_bend_ranges();
            let midi = crate::formats::audio::convert_seq_to_midi(&seq, &ranges)?;
            let out_path = dir.join(format!("{}.mid", stem));
            std::fs::write(&out_path, midi)?;
            println!("  {}: sequence with {} events", stem, seq.events.len());
            written += 1;
            continue;
        }

        // A music font: the bank of its sequences, one MIDI file per track.
        let vhb = match crate::formats::vab2::Vhb::from_bytes(&data) {
            Ok(vhb) => vhb,
            Err(_) => {
                if files.len() == 1 {
                    return Err(crate::error::DwExtractError::InvalidFormat(
                        "the file is not a music font with sequences nor a SEQ".into(),
                    ));
                }
                continue;
            }
        };
        if vhb.seq_data.is_empty() {
            println!("  {}: sound bank without sequences", stem);
            continue;
        }
        let ranges = crate::formats::audio::pitch_bend_ranges(&vhb.vab);
        let tracks = crate::formats::audio::convert_seq_bank_to_midi(&vhb.seq_data, &ranges)?;
        for (track, midi) in tracks.iter().enumerate() {
            let out_path = dir.join(format!("track_{:02}.mid", track));
            std::fs::write(&out_path, midi)?;
            written += 1;
        }
        println!("  {}: {} tracks", stem, tracks.len());
    }

    if written == 0 {
        return Err(crate::error::DwExtractError::InvalidFormat(
            "no sequence found: the input is not a music font with sequences nor a SEQ".into(),
        ));
    }

    println!("Wrote {} MIDI files: {}", written, output.display());
    Ok(())
}

fn run_convert_efe(input: &PathBuf, output: &PathBuf, extract_assets: bool, format: &str) -> Result<()> {
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }
    
    if format != "yaml" && format != "json" {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Unsupported format: {}. Use yaml or json", format)));
    }
    
    std::fs::create_dir_all(output)?;
    let data = std::fs::read(input)?;
    
    let efe = crate::formats::effect::parse_efe_file(&data)?;
    println!("Parsed EFE file: {} slots ({} used)", efe.slots.len(), efe.used_slots().len());
    
    for slot in efe.used_slots() {
        println!("  Slot {}: effect_id=0x{:04X}, spawns={}, sub_fx={}, TMD={:?}, TIM={:?}", 
            slot.index, slot.header.effect_id, slot.header.spawn_count, slot.header.num_sub_effects,
            if !slot.tmd_data().is_empty() { "yes" } else { "no" },
            if !slot.tim_data().is_empty() { "yes" } else { "no" });
    }
    
    if format == "yaml" {
        crate::formats::effect::write_efe_yaml(output.join("efe.yaml"), &efe, extract_assets)?;
    } else {
        // JSON export
        let yaml = crate::formats::effect::efe_file_to_yaml(&efe, extract_assets)?;
        let json = serde_json::to_string_pretty(&yaml)
            .map_err(|e| crate::error::DwExtractError::Json(e))?;
        std::fs::write(output.join("efe.json"), json)?;
    }
    
    if extract_assets {
        crate::formats::effect::extract_efe_assets(&efe, output)?;
        println!("Extracted assets to {}", output.display());
    }
    
    println!("EFE conversion complete: {}", output.display());
    Ok(())
}

fn run_convert_scn(input: &PathBuf, output: &PathBuf, format: &str) -> Result<()> {
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }

    if format != "yaml" && format != "disasm" {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Unsupported format: {}. Use yaml or disasm", format)));
    }

    std::fs::create_dir_all(output)?;
    let data = std::fs::read(input)?;

    // A file of scripts starts with a table of offsets; a single script does not.
    let scripts = match crate::formats::script::read_script_table(&data) {
        Ok(slots) => slots
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                let bytes = data[slot.offset as usize..(slot.offset + slot.size) as usize].to_vec();
                (format!("script_{:04}", index), bytes)
            })
            .collect(),
        Err(_) => vec![("script".to_string(), data.clone())],
    };

    for (name, script) in &scripts {
        let disassembly = crate::formats::script::disassemble_script(script)?;

        println!(
            "{}: {} instructions, {} sections, {} unknown(s)",
            name,
            disassembly.instructions.len(),
            disassembly.header.sections.len(),
            disassembly.unknowns.len()
        );

        if format == "yaml" {
            let yaml_path = output.join(format!("{}.yaml", name));
            crate::formats::script::write_script_yaml(&yaml_path, &disassembly)?;
            println!("Exported YAML: {}", yaml_path.display());
        } else {
            let disasm_path = output.join(format!("{}.disasm", name));
            let listing = crate::formats::script::disassembly::disassembly_to_text(&disassembly);
            std::fs::write(&disasm_path, listing)?;
            println!("Exported disassembly: {}", disasm_path.display());
        }
    }

    println!("SCN conversion complete: {}", output.display());
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

fn run_convert_str(input: &PathBuf, output: &PathBuf, extract_video: bool, extract_audio: bool) -> Result<()> {
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }
    
    if !extract_video && !extract_audio {
        return Err(crate::error::DwExtractError::InvalidBin("Must specify at least --extract-video or --extract-audio".to_string()));
    }
    
    std::fs::create_dir_all(output)?;
    let data = std::fs::read(input)?;
    
    let str_file = crate::formats::video::parse_str_file(&data)?;
    
    println!("Parsed STR file: {} sectors, {} frames, {} audio tracks", 
        str_file.sectors.len(), str_file.frames.len(), str_file.audio_tracks.len());
    
    if extract_video && !str_file.frames.is_empty() {
        let video_dir = output.join("video");
        str_file.extract_video(&video_dir)?;
    }
    
    if extract_audio && !str_file.audio_tracks.is_empty() {
        let audio_dir = output.join("audio");
        str_file.extract_audio(&audio_dir)?;
    }
    
    if extract_video && str_file.frames.is_empty() {
        println!("No video frames found in STR file");
    }
    
    if extract_audio && str_file.audio_tracks.is_empty() {
        println!("No audio tracks found in STR file");
    }
    
    println!("STR conversion complete: {}", output.display());
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
    println!("  ALLTIM.TIM           - Main texture archive (~3.2 MB)");
    println!("  *.MMD                - Digimon model files (178 files)");
    
    Ok(())
}

fn run_convert_map(
    input: &PathBuf, 
    output: &PathBuf, 
    extract_textures: bool, 
    format: &str, 
    images_8: Option<i32>, 
    images_4: Option<i32>, 
    npcs: Option<bool>
) -> Result<()> {
    if !input.exists() {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Input path does not exist: {}", input.display())));
    }
    
    if format != "yaml" && format != "obj" && format != "both" {
        return Err(crate::error::DwExtractError::InvalidBin(format!("Unsupported format: {}. Use yaml, obj, or both", format)));
    }
    
    std::fs::create_dir_all(output)?;
    let data = std::fs::read(input)?;
    
    // Auto-detect options if not provided
    let mut cursor = std::io::Cursor::new(&data);
    let options = if images_8.is_some() && images_4.is_some() && npcs.is_some() {
        crate::formats::map::MapReadOptions::new(images_8.unwrap(), images_4.unwrap(), npcs.unwrap())
    } else {
        crate::formats::map::infer_map_read_options(&mut cursor)?
    };
    
    println!("Map options: {} 8bpp images, {} 4bpp images, NPCs: {}", 
        options.image_count_8, options.image_count_4, options.has_npcs);
    
    let map = crate::formats::map::parse_map_file(&data, &options)?;
    
    println!("Parsed MAP: {}x{} tiles, {} 8bpp + {} 4bpp images, {} sprites, {} instances, {} NPCs, {} warps",
        map.setup.width, map.setup.height,
        options.image_count_8, options.image_count_4,
        map.objects.as_ref().map(|o| o.sprites.len()).unwrap_or(0),
        map.objects.as_ref().map(|o| o.instances.len()).unwrap_or(0),
        map.entities.npcs.len(),
        10
    );
    
    if format == "yaml" || format == "both" {
        crate::formats::map::write_map_yaml(output.join("map.yaml"), &map)?;
        println!("Exported YAML: {}", output.join("map.yaml").display());
    }
    
    if format == "obj" || format == "both" {
        crate::formats::map::export_map_obj(&map, output, extract_textures)?;
        println!("Exported OBJ to: {}", output.display());
    }
    
    println!("MAP conversion complete: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_build_extractor_honors_verify_formats() {
        let dir = tempfile::tempdir().unwrap();

        let verifying = build_extractor(dir.path(), false, true, None, false, None).unwrap();
        assert!(verifying.verify_formats);

        let skipping = build_extractor(dir.path(), false, false, None, false, None).unwrap();
        assert!(!skipping.verify_formats);
    }

    #[test]
    fn test_build_extractor_without_reference_list() {
        let dir = tempfile::tempdir().unwrap();
        let extractor = build_extractor(dir.path(), false, true, None, false, None).unwrap();
        assert!(!extractor.has_reference());
    }

    #[test]
    fn test_build_extractor_with_reference_list() {
        let dir = tempfile::tempdir().unwrap();

        let list_path = dir.path().join("redump.txt");
        let mut list = std::fs::File::create(&list_path).unwrap();
        list.write_all(b"a9993e364706816aba3e25717850c26c9cd0d89d,3,ABC.BIN\n")
            .unwrap();
        list.flush().unwrap();

        let extractor = build_extractor(dir.path(), false, true, None, false, Some(&list_path)).unwrap();
        assert!(extractor.has_reference());
    }
}
