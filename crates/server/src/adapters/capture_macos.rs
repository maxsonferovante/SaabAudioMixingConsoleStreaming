use audio_core::domain::AudioBuffer;
use audio_core::error::CoreError;
use audio_core::ports::secondary::AudioCapturePort;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info};

pub struct MacAudioCapture {
    stream: Option<cpal::Stream>,
    running: Arc<AtomicBool>,
}

impl Default for MacAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl MacAudioCapture {
    pub fn new() -> Self {
        Self { stream: None, running: Arc::new(AtomicBool::new(false)) }
    }
}

impl AudioCapturePort for MacAudioCapture {
    fn start_capture(
        &mut self,
        mut callback: Box<dyn FnMut(AudioBuffer) + Send + 'static>,
    ) -> Result<(), CoreError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            CoreError::CaptureError("No default input audio device found on macOS".into())
        })?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown Device".into());
        info!("Initializing audio capture on device: {}", device_name);

        let default_config = device.default_input_config().map_err(|e| {
            CoreError::CaptureError(format!("Failed to get default input config: {:?}", e))
        })?;

        let config: StreamConfig = default_config.into();
        let input_channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;

        self.running.store(true, Ordering::SeqCst);
        let running_flag = Arc::clone(&self.running);

        let chunk_size = 240; // 5ms frames
        let mut sample_accumulator = Vec::with_capacity(chunk_size * 2);

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !running_flag.load(Ordering::Relaxed) {
                        return;
                    }

                    for frame in data.chunks(input_channels) {
                        let (left, right) = match frame.len() {
                            0 => (0.0, 0.0),
                            1 => (frame[0], frame[0]),
                            _ => (frame[0], frame[1]),
                        };

                        sample_accumulator.push(left);
                        sample_accumulator.push(right);

                        if sample_accumulator.len() >= chunk_size * 2 {
                            if let Ok(buffer) = AudioBuffer::new(
                                std::mem::replace(
                                    &mut sample_accumulator,
                                    Vec::with_capacity(chunk_size * 2),
                                ),
                                2,
                                sample_rate,
                            ) {
                                callback(buffer);
                            }
                        }
                    }
                },
                move |err| {
                    error!("macOS Audio Capture stream error: {:?}", err);
                },
                None,
            )
            .map_err(|e| {
                CoreError::CaptureError(format!("Failed to build input stream: {:?}", e))
            })?;

        stream.play().map_err(|e| {
            CoreError::CaptureError(format!("Failed to play capture stream: {:?}", e))
        })?;

        self.stream = Some(stream);
        info!("macOS Audio Capture running at {}Hz", sample_rate);

        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), CoreError> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
        info!("macOS Audio Capture stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_audio_capture_initialization() {
        let capture = MacAudioCapture::new();
        assert!(!capture.running.load(Ordering::Relaxed));
    }
}
