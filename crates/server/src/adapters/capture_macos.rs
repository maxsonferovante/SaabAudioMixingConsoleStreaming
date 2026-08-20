use audio_core::domain::AudioBuffer;
use audio_core::error::CoreError;
use audio_core::ports::secondary::AudioCapturePort;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

#[cfg(target_os = "macos")]
use screencapturekit::cm::{CMSampleBuffer, CMSampleBufferExt};
#[cfg(target_os = "macos")]
use screencapturekit::prelude::{
    SCContentFilter, SCShareableContent, SCStream, SCStreamConfiguration, SCStreamOutputType,
};

type SharedAudioCallback = Arc<std::sync::Mutex<Box<dyn FnMut(AudioBuffer) + Send + 'static>>>;

pub struct MacAudioCapture {
    cpal_stream: Option<cpal::Stream>,
    #[cfg(target_os = "macos")]
    sck_stream: Option<SCStream>,
    running: Arc<AtomicBool>,
    device_name: Option<String>,
}

impl Default for MacAudioCapture {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MacAudioCapture {
    pub fn new(device_name: Option<String>) -> Self {
        Self {
            cpal_stream: None,
            #[cfg(target_os = "macos")]
            sck_stream: None,
            running: Arc::new(AtomicBool::new(false)),
            device_name,
        }
    }

    #[cfg(target_os = "macos")]
    fn try_start_screencapturekit(
        &mut self,
        callback: SharedAudioCallback,
    ) -> Result<(), CoreError> {
        info!("Attempting Apple ScreenCaptureKit system audio capture (native, zero-driver)...");

        let content = SCShareableContent::get().map_err(|e| {
            CoreError::CaptureError(format!(
                "SCShareableContent::get failed (check Screen/Audio Recording permissions): {:?}",
                e
            ))
        })?;

        let displays = content.displays();
        let main_display = displays.first().ok_or_else(|| {
            CoreError::CaptureError("No display found for ScreenCaptureKit".into())
        })?;

        let filter = SCContentFilter::create()
            .with_display(main_display)
            .with_excluding_windows(&[])
            .build();

        let config = SCStreamConfiguration::new()
            .with_width(100)
            .with_height(100)
            .with_captures_audio(true)
            .with_sample_rate(48_000)
            .with_channel_count(2);

        let mut stream = SCStream::new(&filter, &config);

        let running_flag = Arc::clone(&self.running);
        let cb_clone = Arc::clone(&callback);

        stream.add_output_handler(
            move |sample: CMSampleBuffer, of_type: SCStreamOutputType| {
                if !running_flag.load(Ordering::Relaxed) {
                    return;
                }

                if of_type == SCStreamOutputType::Audio {
                    if let Some(audio_list) = sample.audio_buffer_list() {
                        let buffers: Vec<_> = audio_list.iter().collect();
                        if buffers.is_empty() {
                            return;
                        }

                        let interleaved_samples: Vec<f32> = if buffers.len() == 1 {
                            let buf = &buffers[0];
                            let raw = buf.data();
                            let floats: Vec<f32> = raw
                                .chunks_exact(4)
                                .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                                .collect();

                            if buf.number_channels == 1 {
                                // Mono to Stereo expansion
                                let mut stereo = Vec::with_capacity(floats.len() * 2);
                                for s in floats {
                                    stereo.push(s);
                                    stereo.push(s);
                                }
                                stereo
                            } else {
                                floats
                            }
                        } else {
                            // Non-interleaved stereo
                            let left_raw = buffers[0].data();
                            let right_raw = buffers[1].data();

                            let left_floats: Vec<f32> = left_raw
                                .chunks_exact(4)
                                .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                                .collect();
                            let right_floats: Vec<f32> = right_raw
                                .chunks_exact(4)
                                .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                                .collect();

                            let count = left_floats.len().min(right_floats.len());
                            let mut stereo = Vec::with_capacity(count * 2);
                            for i in 0..count {
                                stereo.push(left_floats[i]);
                                stereo.push(right_floats[i]);
                            }
                            stereo
                        };

                        if !interleaved_samples.is_empty() {
                            if let Ok(audio_buf) = AudioBuffer::new(interleaved_samples, 2, 48000) {
                                if let Ok(mut lock) = cb_clone.lock() {
                                    lock(audio_buf);
                                }
                            }
                        }
                    }
                }
            },
            SCStreamOutputType::Audio,
        );

        stream.start_capture().map_err(|e| {
            CoreError::CaptureError(format!("SCStream::start_capture failed: {:?}", e))
        })?;

        self.sck_stream = Some(stream);
        info!("Apple ScreenCaptureKit system audio stream active at 48000Hz stereo.");

        Ok(())
    }

    fn start_cpal_fallback(&mut self, callback: SharedAudioCallback) -> Result<(), CoreError> {
        let host = cpal::default_host();

        let input_devices: Vec<_> = host
            .input_devices()
            .map_err(|e| {
                CoreError::CaptureError(format!("Failed to enumerate input devices: {:?}", e))
            })?
            .collect();

        info!("Available macOS Audio Input Devices:");
        for (i, dev) in input_devices.iter().enumerate() {
            let name = dev.name().unwrap_or_else(|_| "Unknown".into());
            info!("  [{}] {}", i, name);
        }

        let target_device_name =
            self.device_name.clone().or_else(|| std::env::var("AUDIO_DEVICE").ok());

        let device = if let Some(ref target_name) = target_device_name {
            input_devices.into_iter().find(|d| {
                d.name()
                    .map(|n| n.to_lowercase().contains(&target_name.to_lowercase()))
                    .unwrap_or(false)
            })
        } else {
            let loopback_dev = input_devices.into_iter().find(|d| {
                d.name()
                    .map(|n| {
                        let lower = n.to_lowercase();
                        lower.contains("blackhole")
                            || lower.contains("loopback")
                            || lower.contains("soundflower")
                            || lower.contains("aggregate")
                            || lower.contains("multi-output")
                    })
                    .unwrap_or(false)
            });

            loopback_dev.or_else(|| host.default_input_device())
        }
        .ok_or_else(|| {
            CoreError::CaptureError("No suitable audio input device found on macOS".into())
        })?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown Device".into());
        info!("Selected audio capture device: {}", device_name);

        let default_config = device.default_input_config().map_err(|e| {
            CoreError::CaptureError(format!("Failed to get default input config: {:?}", e))
        })?;

        let config: StreamConfig = default_config.into();
        let input_channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;

        let running_flag = Arc::clone(&self.running);
        let chunk_size = 240;
        let mut sample_accumulator = Vec::with_capacity(chunk_size * 2);
        let cb_clone = Arc::clone(&callback);

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
                                if let Ok(mut lock) = cb_clone.lock() {
                                    lock(buffer);
                                }
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

        self.cpal_stream = Some(stream);
        info!("macOS Audio Capture running at {}Hz ({} channels)", sample_rate, input_channels);

        Ok(())
    }
}

impl AudioCapturePort for MacAudioCapture {
    fn start_capture(
        &mut self,
        callback: Box<dyn FnMut(AudioBuffer) + Send + 'static>,
    ) -> Result<(), CoreError> {
        self.running.store(true, Ordering::SeqCst);
        let shared_callback = Arc::new(std::sync::Mutex::new(callback));

        #[cfg(target_os = "macos")]
        {
            // If the user did not explicitly request a specific physical input device, try native ScreenCaptureKit first
            if self.device_name.is_none() {
                match self.try_start_screencapturekit(Arc::clone(&shared_callback)) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        warn!("ScreenCaptureKit initialization failed ({:?}). Falling back to hardware/loopback audio...", e);
                    }
                }
            }
        }

        self.start_cpal_fallback(shared_callback)
    }

    fn stop_capture(&mut self) -> Result<(), CoreError> {
        self.running.store(false, Ordering::SeqCst);

        #[cfg(target_os = "macos")]
        {
            if let Some(stream) = self.sck_stream.take() {
                let _ = stream.stop_capture();
            }
        }

        if let Some(stream) = self.cpal_stream.take() {
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
        let capture = MacAudioCapture::new(None);
        assert!(!capture.running.load(Ordering::Relaxed));
    }
}
