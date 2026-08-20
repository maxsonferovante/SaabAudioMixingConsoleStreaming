use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum CoreError {
    #[error("Invalid audio buffer: {0}")]
    InvalidAudioBuffer(String),
    #[error("Capture error: {0}")]
    CaptureError(String),
    #[error("Playback error: {0}")]
    PlaybackError(String),
    #[error("Streaming error: {0}")]
    StreamingError(String),
    #[error("Telemetry error: {0}")]
    TelemetryError(String),
}
