use audio_core::domain::AudioBuffer;
use audio_core::error::CoreError;
use audio_core::ports::secondary::AudioCapturePort;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

type SharedAudioCallback = Arc<std::sync::Mutex<Box<dyn FnMut(AudioBuffer) + Send + 'static>>>;

pub struct MacAudioCapture {
    cpal_stream: Option<cpal::Stream>,
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
        Self { cpal_stream: None, running: Arc::new(AtomicBool::new(false)), device_name }
    }

    /// Resolves the optimal virtual audio device or hardware input
    fn resolve_device(
        host: &cpal::Host,
        override_name: Option<&str>,
    ) -> Result<cpal::Device, CoreError> {
        let input_devices: Vec<_> = host
            .input_devices()
            .map_err(|e| {
                CoreError::CaptureError(format!("Failed to enumerate input devices: {:?}", e))
            })?
            .collect();

        info!("Available macOS Audio Input Devices (CoreAudio HAL):");
        for (i, dev) in input_devices.iter().enumerate() {
            let name = dev.name().unwrap_or_else(|_| "Unknown".into());
            info!("  [{}] {}", i, name);
        }

        // Priority 1: User override via CLI or Environment
        if let Some(target) = override_name {
            if let Some(dev) = input_devices.iter().find(|d| {
                d.name().map(|n| n.to_lowercase().contains(&target.to_lowercase())).unwrap_or(false)
            }) {
                return Ok(dev.clone());
            }
            warn!("Specified audio device '{}' not found, falling back to auto-discovery", target);
        }

        // Priority 2: BlackHole 16ch (Project Default)
        if let Some(dev) = input_devices.iter().find(|d| {
            d.name().map(|n| n.to_lowercase().contains("blackhole 16ch")).unwrap_or(false)
        }) {
            return Ok(dev.clone());
        }

        // Priority 3: BlackHole 2ch
        if let Some(dev) = input_devices
            .iter()
            .find(|d| d.name().map(|n| n.to_lowercase().contains("blackhole 2ch")).unwrap_or(false))
        {
            return Ok(dev.clone());
        }

        // Priority 4: BlackHole 64ch
        if let Some(dev) = input_devices.iter().find(|d| {
            d.name().map(|n| n.to_lowercase().contains("blackhole 64ch")).unwrap_or(false)
        }) {
            return Ok(dev.clone());
        }

        // Priority 5: Generic BlackHole or Loopback
        if let Some(dev) = input_devices.iter().find(|d| {
            d.name()
                .map(|n| {
                    let l = n.to_lowercase();
                    l.contains("blackhole") || l.contains("loopback") || l.contains("multi-output")
                })
                .unwrap_or(false)
        }) {
            return Ok(dev.clone());
        }

        // Priority 6: Default input device with explicit setup guidance
        warn!("==========================================================================");
        warn!("[WARNING] BlackHole 16ch virtual audio driver not detected on this system.");
        warn!("To route and capture system audio digitally with zero latency, please install:");
        warn!("  brew install blackhole-16ch   (Project Standard - 16-channel DAWs & surround)");
        warn!("  brew install blackhole-2ch    (Alternative - 2-channel basic stereo)");
        warn!("  brew install blackhole-64ch   (Alternative - 64-channel studio routing)");
        warn!("After installing, set your macOS Output to BlackHole in System Settings -> Sound.");
        warn!("==========================================================================");

        host.default_input_device()
            .ok_or_else(|| CoreError::CaptureError("No audio input device found on macOS".into()))
    }
}

/// Applies ITU-R BS.775 broadcast downmixing to a multi-channel frame
pub fn downmix_frame_to_stereo(frame: &[f32]) -> (f32, f32) {
    match frame.len() {
        0 => (0.0, 0.0),
        1 => (frame[0], frame[0]),
        2 => (frame[0], frame[1]),
        6 => {
            // 5.1 Surround Downmix (ITU-R BS.775)
            // Ch 0: L, Ch 1: R, Ch 2: C, Ch 3: LFE, Ch 4: Ls, Ch 5: Rs
            let c = frame[2] * FRAC_1_SQRT_2;
            let l = frame[0] + c + frame[4] * FRAC_1_SQRT_2;
            let r = frame[1] + c + frame[5] * FRAC_1_SQRT_2;
            (l, r)
        }
        _ => {
            // 16ch or multi-channel: extract main L/R (ch 0 & 1)
            // with surround fold-down if active
            if frame.len() >= 6 && (frame[2].abs() > 0.001 || frame[4].abs() > 0.001) {
                let c = frame[2] * FRAC_1_SQRT_2;
                let l = frame[0] + c + frame[4] * FRAC_1_SQRT_2;
                let r = frame[1] + c + frame[5] * FRAC_1_SQRT_2;
                (l, r)
            } else {
                (frame[0], frame[1])
            }
        }
    }
}

impl AudioCapturePort for MacAudioCapture {
    fn start_capture(
        &mut self,
        callback: Box<dyn FnMut(AudioBuffer) + Send + 'static>,
    ) -> Result<(), CoreError> {
        let callback_arc: SharedAudioCallback = Arc::new(std::sync::Mutex::new(callback));
        self.running.store(true, Ordering::SeqCst);

        let host = cpal::default_host();
        let target_override =
            self.device_name.clone().or_else(|| std::env::var("AUDIO_DEVICE").ok());

        let device = Self::resolve_device(&host, target_override.as_deref())?;
        let device_name = device.name().unwrap_or_else(|_| "Unknown Device".into());
        info!("CoreAudio HAL Capture active on device: {}", device_name);

        let default_config = device.default_input_config().map_err(|e| {
            CoreError::CaptureError(format!("Failed to get default input config: {:?}", e))
        })?;

        let config: StreamConfig = default_config.into();
        let input_channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;

        info!(
            "BlackHole Audio Capture format: {}Hz, {} input channels",
            sample_rate, input_channels
        );

        let running_flag = Arc::clone(&self.running);
        // Size chunk proportionally to sample rate (5ms chunks: 240 frames at 48kHz, 480 frames at 96kHz, 960 at 192kHz)
        let chunk_frames = (sample_rate as usize * 5) / 1000;
        let mut sample_accumulator = Vec::with_capacity(chunk_frames * 2);
        let cb_clone = Arc::clone(&callback_arc);

        let err_fn = |err| error!("An error occurred on the audio input stream: {:?}", err);

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !running_flag.load(Ordering::Relaxed) {
                        return;
                    }

                    for frame in data.chunks(input_channels) {
                        let (left, right) = downmix_frame_to_stereo(frame);

                        sample_accumulator.push(left);
                        sample_accumulator.push(right);

                        if sample_accumulator.len() >= chunk_frames * 2 {
                            let block_samples = std::mem::replace(
                                &mut sample_accumulator,
                                Vec::with_capacity(chunk_frames * 2),
                            );

                            static BLOCK_COUNTER: std::sync::atomic::AtomicU64 =
                                std::sync::atomic::AtomicU64::new(0);
                            let count = BLOCK_COUNTER.fetch_add(1, Ordering::Relaxed);
                            if count == 0 || count % 500 == 0 {
                                info!(
                                    "BlackHole Capture: processed block #{} ({} samples, {}Hz)",
                                    count,
                                    block_samples.len() / 2,
                                    sample_rate
                                );
                            }

                            if let Ok(buffer) = AudioBuffer::new(block_samples, 2, sample_rate) {
                                if let Ok(mut lock) = cb_clone.lock() {
                                    lock(buffer);
                                }
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| {
                CoreError::CaptureError(format!("Failed to build CPAL input stream: {:?}", e))
            })?;

        stream.play().map_err(|e| {
            CoreError::CaptureError(format!("Failed to play CPAL input stream: {:?}", e))
        })?;

        self.cpal_stream = Some(stream);
        info!("macOS Audio Capture running at {}Hz (bit-exact CoreAudio HAL)", sample_rate);

        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), CoreError> {
        self.running.store(false, Ordering::SeqCst);
        self.cpal_stream = None;
        info!("macOS Audio Capture stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_audio_capture_initialization() {
        let capture = MacAudioCapture::new(Some("Test Device".into()));
        assert!(!capture.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_downmix_stereo_passthrough() {
        let frame = [0.5, -0.5];
        let (l, r) = downmix_frame_to_stereo(&frame);
        assert!((l - 0.5).abs() < 1e-6);
        assert!((r - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_downmix_mono_duplication() {
        let frame = [0.75];
        let (l, r) = downmix_frame_to_stereo(&frame);
        assert!((l - 0.75).abs() < 1e-6);
        assert!((r - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_downmix_5_1_surround_itur_bs775() {
        // L=0.2, R=0.3, C=0.4, LFE=0.0, Ls=0.1, Rs=0.1
        let frame = [0.2, 0.3, 0.4, 0.0, 0.1, 0.1];
        let (l, r) = downmix_frame_to_stereo(&frame);
        let expected_c = 0.4 * FRAC_1_SQRT_2;
        let expected_l = 0.2 + expected_c + 0.1 * FRAC_1_SQRT_2;
        let expected_r = 0.3 + expected_c + 0.1 * FRAC_1_SQRT_2;
        assert!((l - expected_l).abs() < 1e-5);
        assert!((r - expected_r).abs() < 1e-5);
    }

    #[test]
    fn test_downmix_16_channel_frame() {
        let mut frame = [0.0; 16];
        frame[0] = 0.8;
        frame[1] = -0.4;
        let (l, r) = downmix_frame_to_stereo(&frame);
        assert!((l - 0.8).abs() < 1e-6);
        assert!((r - (-0.4)).abs() < 1e-6);
    }
}
