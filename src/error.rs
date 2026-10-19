use thiserror::Error;

#[derive(Error, Debug)]
pub enum DwExtractError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid CUE file: {0}")]
    InvalidCue(String),

    #[error("Invalid BIN file: {0}")]
    InvalidBin(String),

    #[error("Sector read error at sector {sector}: {message}")]
    SectorRead { sector: u32, message: String },

    #[error("File not found in file table: {0}")]
    FileNotFound(String),

    #[error("File table error: {0}")]
    FileTable(String),

    #[error("Extraction error for {file}: {message}")]
    Extraction { file: String, message: String },

    #[error("Hash mismatch for {file}: expected {expected}, got {actual}")]
    HashMismatch { file: String, expected: String, actual: String },

    #[error("Redump verification failed: {0}")]
    RedumpVerification(String),

    #[error("Unsupported game version: {0}")]
    UnsupportedVersion(String),

    #[error("Output file exists and differs: {0}. Use --force to overwrite.")]
    OutputExists(String),

    #[error("Invalid TIM format: {0}")]
    InvalidTim(String),

    #[error("Invalid MMD format: {0}")]
    InvalidMmd(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Binrw error: {0}")]
    Binrw(#[from] binrw::Error),

    #[error("Nom parse error: {0}")]
    Nom(String),
}

impl From<nom::Err<nom::error::Error<&[u8]>>> for DwExtractError {
    fn from(e: nom::Err<nom::error::Error<&[u8]>>) -> Self {
        DwExtractError::Nom(format!("{:?}", e))
    }
}

pub type Result<T> = std::result::Result<T, DwExtractError>;