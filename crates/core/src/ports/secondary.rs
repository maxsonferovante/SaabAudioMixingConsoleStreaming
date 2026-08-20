use crate::domain::{AudioBuffer, VuMeterReading};
use crate::error::CoreError;

/// Driven Port for streaming UDP audio packets over the network
pub trait AudioStreamerPort: Send + Sync {
    fn stream_audio(
        &self,
        buffer: &AudioBuffer,
        sequence_number: u64,
        timestamp_us: u64,
    ) -> Result<(), CoreError>;
}

/// Driven Port for broadcasting telemetry and VU meter data to connected clients
pub trait TelemetryBroadcasterPort: Send + Sync {
    fn broadcast_vu(&self, reading: &VuMeterReading) -> Result<(), CoreError>;
}

/// Driven Port for playing audio to physical hardware / audio output devices
pub trait AudioPlaybackPort: Send {
    fn write_samples(&mut self, samples: &[f32]) -> Result<usize, CoreError>;
}

/// Driving/Driven Port for capturing digital audio from the OS or audio hardware
pub trait AudioCapturePort: Send {
    fn start_capture(
        &mut self,
        callback: Box<dyn FnMut(AudioBuffer) + Send + 'static>,
    ) -> Result<(), CoreError>;
    fn stop_capture(&mut self) -> Result<(), CoreError>;
}
