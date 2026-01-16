use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum Error {
    #[error("Invalid YouTube URL or video ID: {0}")]
    InvalidUrl(String),

    #[error("Failed to extract video info: {0}")]
    ExtractionFailed(String),

    #[error("No formats available")]
    NoFormats,

    #[error("Format not found: {0}")]
    FormatNotFound(String),

    #[error("Signature decryption failed: {0}")]
    SignatureFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Video unavailable: {0}")]
    VideoUnavailable(String),
}

pub type Result<T> = std::result::Result<T, Error>;
