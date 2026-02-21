use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QsmxtError {
    #[error("BIDS discovery error: {0}")]
    BidsDiscovery(String),

    #[error("No phase files found for {subject}{session}")]
    NoPhaseFiles { subject: String, session: String },

    #[error("JSON sidecar missing required field '{field}' in {path}")]
    MissingSidecarField { field: String, path: PathBuf },

    #[error("JSON sidecar parse error for {path}: {source}")]
    SidecarParse {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("NIfTI I/O error: {0}")]
    NiftiIo(String),

    #[error("Dimension mismatch: {0}")]
    DimensionMismatch(String),

    #[error("Pipeline configuration error: {0}")]
    Config(String),

    #[error("Algorithm error in {stage}: {message}")]
    Algorithm { stage: String, message: String },

    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SLURM submission error: {0}")]
    Slurm(String),
}

pub type Result<T> = std::result::Result<T, QsmxtError>;
