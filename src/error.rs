use thiserror::Error;

#[derive(Error, Debug)]
pub enum SonicPipeError {
    #[error("Audio device error: {0}")]
    AudioDevice(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Decoding error: {0}")]
    Decoding(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("ECC error: {0}")]
    ErrorCorrection(String),

    #[error("Invalid packet: {0}")]
    InvalidPacket(String),

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    #[error("No wake-up tone detected")]
    NoWakeUpTone,

    #[error("Timeout waiting for data")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SonicPipeError>;
