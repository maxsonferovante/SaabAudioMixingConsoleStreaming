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
        Self { running: Arc::new(AtomicBool::new(false)), device_name }
    }

    /// Resolves the optimal virtual audio device or hardware input matching the active output
    pub fn resolve_active_device(
        host: &cpal::Host,
        override_name: Option<&str>,
    ) -> Result<cpal::Device, CoreError> {
        let input_devices: Vec<_> = host
            .input_devices()
            .map_err(|e| {
                CoreError::CaptureError(format!("Failed to enumerate input devices: {:?}", e))
            })?
            .collect();

        // Priority 1: Explicit CLI or Environment override (if not "auto")
        if let Some(target) = override_name {
            if target != "auto" && !target.is_empty() {
                if let Some(dev) = input_devices.iter().find(|d| {
                    d.name()
                        .map(|n| n.to_lowercase().contains(&target.to_lowercase()))
                        .unwrap_or(false)
                }) {
                    return Ok(dev.clone());
                }
                warn!(
                    "Specified audio device '{}' not found, falling back to dynamic auto-discovery",
                    target
                );
            }
        }

        // Priority 2: Match active macOS Default Output if it is a BlackHole variant
        if let Some(default_out) = host.default_output_device() {
            if let Ok(out_name) = default_out.name() {
                let out_lower = out_name.to_lowercase();
                if out_lower.contains("blackhole") {
                    if let Some(dev) = input_devices.iter().find(|d| {
                        d.name().map(|n| n.to_lowercase().contains(&out_lower)).unwrap_or(false)
                    }) {
                        return Ok(dev.clone());
                    }
                }
            }
        }

        // Priority 3: BlackHole 16ch (Project Standard for 16-channel DAWs & Surround)
        if let Some(dev) = input_devices.iter().find(|d| {
            d.name().map(|n| n.to_lowercase().contains("blackhole 16ch")).unwrap_or(false)
        }) {
            return Ok(dev.clone());
        }

        // Priority 4: BlackHole 2ch (Universal Standard for Stereo Media)
        if let Some(dev) = input_devices
            .iter()
            .find(|d| d.name().map(|n| n.to_lowercase().contains("blackhole 2ch")).unwrap_or(false))
        {
            return Ok(dev.clone());
        }

        // Priority 5: BlackHole 64ch
        if let Some(dev) = input_devices.iter().find(|d| {
            d.name().map(|n| n.to_lowercase().contains("blackhole 64ch")).unwrap_or(false)
        }) {
            return Ok(dev.clone());
        }

        // Priority 6: Generic BlackHole, Loopback, or Multi-Output
        if let Some(dev) = input_devices.iter().find(|d| {
            d.name()
                .map(|n| {
                    let l = n.to_lowercase();
                    l.contains("blackhole")
                        || l.contains("loopback")
                        || l.contains("soundflower")
                        || l.contains("multi-output")
                })
                .unwrap_or(false)
        }) {
            return Ok(dev.clone());
        }

        // Priority 7: Default input device with explicit setup guidance
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

    /// Builds and starts a CPAL input stream on a specific device with dedicated cancellation token
    fn build_and_start_stream(
        device: &cpal::Device,
        stream_running_flag: Arc<AtomicBool>,
        callback_arc: SharedAudioCallback,
    ) -> Result<cpal::Stream, CoreError> {
        let dev_name = device.name().unwrap_or_else(|_| "Unknown Device".into());
        let default_config = device.default_input_config().map_err(|e| {
            CoreError::CaptureError(format!("Failed to get default input config: {:?}", e))
        })?;

        let config: StreamConfig = default_config.into();
        let input_channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;

        info!(
            "Initializing capture on '{}': {}Hz, {} input channels",
            dev_name, sample_rate, input_channels
        );

        let chunk_frames = (sample_rate as usize * 5) / 1000;
        let mut sample_accumulator = Vec::with_capacity(chunk_frames * 2);
        let mut channel_peaks = vec![0.0f32; input_channels.max(2)];
        let cb_clone = Arc::clone(&callback_arc);
        let dev_name_clone = dev_name.clone();

        let err_fn = move |err| error!("Error on CoreAudio input stream ({}): {:?}", dev_name, err);

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !stream_running_flag.load(Ordering::Relaxed) {
                        return;
                    }

                    for frame in data.chunks(input_channels) {
                        for (idx, &sample) in frame.iter().enumerate() {
                            if idx < channel_peaks.len() {
                                let abs = sample.abs();
                                if abs > channel_peaks[idx] {
                                    channel_peaks[idx] = abs;
                                }
                            }
                        }

                        let (left, right) = downmix_frame_to_stereo(frame);

                        sample_accumulator.push(left);
                        sample_accumulator.push(right);

                        if sample_accumulator.len() >= chunk_frames * 2 {
                            let block_samples = std::mem::replace(
                                &mut sample_accumulator,
                                Vec::with_capacity(chunk_frames * 2),
                            );

                            let mut peak: f32 = 0.0;
                            for &s in &block_samples {
                                let abs = s.abs();
                                if abs > peak {
                                    peak = abs;
                                }
                            }

                            static BLOCK_COUNTER: std::sync::atomic::AtomicU64 =
                                std::sync::atomic::AtomicU64::new(0);
                            let count = BLOCK_COUNTER.fetch_add(1, Ordering::Relaxed);
                            if count == 0 || count % 500 == 0 {
                                if peak > 0.0001 {
                                    if input_channels >= 16 {
                                        info!(
                                            "{} Capture: block #{} ({} samples, {}Hz, peak: {:.4}) [Ch 0-1: {:.3}, Ch 2-3: {:.3}, Ch 4-5: {:.3}, Ch 6-7: {:.3}]",
                                            dev_name_clone,
                                            count,
                                            block_samples.len() / 2,
                                            sample_rate,
                                            peak,
                                            channel_peaks.first().copied().unwrap_or(0.0).max(channel_peaks.get(1).copied().unwrap_or(0.0)),
                                            channel_peaks.get(2).copied().unwrap_or(0.0).max(channel_peaks.get(3).copied().unwrap_or(0.0)),
                                            channel_peaks.get(4).copied().unwrap_or(0.0).max(channel_peaks.get(5).copied().unwrap_or(0.0)),
                                            channel_peaks.get(6).copied().unwrap_or(0.0).max(channel_peaks.get(7).copied().unwrap_or(0.0)),
                                        );
                                    } else {
                                        info!(
                                            "{} Capture: block #{} ({} samples, {}Hz, peak signal: {:.4})",
                                            dev_name_clone,
                                            count,
                                            block_samples.len() / 2,
                                            sample_rate,
                                            peak
                                        );
                                    }
                                } else {
                                    info!(
                                        "{} Capture: block #{} ({} samples, {}Hz, SILENCE - check macOS Output)",
                                        dev_name_clone,
                                        count,
                                        block_samples.len() / 2,
                                        sample_rate
                                    );
                                }

                                for p in channel_peaks.iter_mut() {
                                    *p = 0.0;
                                }
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

        Ok(stream)
    }
}

/// Applies ITU-R BS.775 broadcast downmixing and soft-limiting to a multi-channel frame
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
            let out_l = if l.abs() > 1.0 { l.tanh() } else { l };
            let out_r = if r.abs() > 1.0 { r.tanh() } else { r };
            (out_l, out_r)
        }
        _ => {
            // Multi-channel (e.g. 16-channel BlackHole / DAW buses)
            // Stereo pairs are mapped as: (Ch 0, Ch 1), (Ch 2, Ch 3), (Ch 4, Ch 5), ..., (Ch 2k, Ch 2k+1)
            let mut l = frame[0];
            let mut r = frame.get(1).copied().unwrap_or(0.0);

            let main_silent =
                frame[0].abs() <= 0.0001 && frame.get(1).copied().unwrap_or(0.0).abs() <= 0.0001;

            let mut i = 2;
            let mut secondary_active = false;

            while i < frame.len() {
                let pair_l = frame[i];
                let pair_r = if i + 1 < frame.len() { frame[i + 1] } else { pair_l };

                if pair_l.abs() > 0.0001 || pair_r.abs() > 0.0001 {
                    if main_silent && !secondary_active {
                        // If main bus is silent, route the primary secondary stereo pair at unity gain
                        l = pair_l;
                        r = pair_r;
                        secondary_active = true;
                    } else {
                        // Blend additional pairs with acoustic power scaling
                        l += pair_l * FRAC_1_SQRT_2;
                        r += pair_r * FRAC_1_SQRT_2;
                    }
                }
                i += 2;
            }

            let out_l = if l.abs() > 1.0 { l.tanh() } else { l };
            let out_r = if r.abs() > 1.0 { r.tanh() } else { r };
            (out_l, out_r)
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

        let running_thread = Arc::clone(&self.running);
        let target_override =
            self.device_name.clone().or_else(|| std::env::var("AUDIO_DEVICE").ok());

        std::thread::spawn(move || {
            info!("CoreAudio HAL Auto-Follower supervisor active (monitoring macOS Sound Output)");
            let host = cpal::default_host();
            let mut active_device_name = String::new();
            let mut active_stream: Option<cpal::Stream> = None;
            let mut active_stream_flag: Option<Arc<AtomicBool>> = None;

            while running_thread.load(Ordering::Relaxed) {
                let target_res = Self::resolve_active_device(&host, target_override.as_deref());

                match target_res {
                    Ok(device) => {
                        let dev_name = device.name().unwrap_or_else(|_| "Unknown Device".into());

                        if dev_name != active_device_name || active_stream.is_none() {
                            if !active_device_name.is_empty() {
                                info!(
                                    "Auto-Follower: Detected macOS Output change ({} -> {}). Migrating capture stream in <100ms...",
                                    active_device_name, dev_name
                                );
                            }

                            // 1. Invalidate previous stream callback immediately
                            if let Some(flag) = active_stream_flag.take() {
                                flag.store(false, Ordering::SeqCst);
                            }

                            // 2. Pause and drop old stream
                            if let Some(stream) = active_stream.take() {
                                let _ = stream.pause();
                                drop(stream);
                            }

                            // 3. Allow CoreAudio HAL 50ms to flush buffers cleanly
                            std::thread::sleep(std::time::Duration::from_millis(50));

                            // 4. Create new dedicated stream running flag
                            let new_stream_flag = Arc::new(AtomicBool::new(true));

                            match Self::build_and_start_stream(
                                &device,
                                Arc::clone(&new_stream_flag),
                                Arc::clone(&callback_arc),
                            ) {
                                Ok(stream) => {
                                    active_device_name = dev_name.clone();
                                    active_stream = Some(stream);
                                    active_stream_flag = Some(new_stream_flag);
                                    info!(
                                        "CoreAudio HAL capture running on '{}' (bit-exact)",
                                        dev_name
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to initialize capture stream on '{}': {:?}",
                                        dev_name, e
                                    );
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Auto-Follower: No suitable audio device available ({:?}), retrying in 500ms...",
                            e
                        );
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(250));
            }

            if let Some(flag) = active_stream_flag.take() {
                flag.store(false, Ordering::SeqCst);
            }
            if let Some(stream) = active_stream.take() {
                let _ = stream.pause();
                drop(stream);
            }
            info!("CoreAudio HAL capture supervisor terminated");
        });

        info!("macOS Audio Capture stream supervisor initialized");
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), CoreError> {
        self.running.store(false, Ordering::SeqCst);
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

    #[test]
    fn test_downmix_16_channel_secondary_bus_ch2_ch3() {
        let mut frame = [0.0; 16];
        frame[2] = 0.6;
        frame[3] = -0.6;
        let (l, r) = downmix_frame_to_stereo(&frame);
        assert!((l - 0.6).abs() < 1e-5);
        assert!((r - (-0.6)).abs() < 1e-5);
    }

    #[test]
    fn test_downmix_16_channel_full_saturation_soft_limiting() {
        let frame = [1.0; 16];
        let (l, r) = downmix_frame_to_stereo(&frame);
        assert!(l > 0.9 && l <= 1.0);
        assert!(r > 0.9 && r <= 1.0);
    }
}
