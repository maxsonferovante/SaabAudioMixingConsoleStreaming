#[cfg(target_os = "android")]
use oboe::{
    AudioOutputCallback, AudioOutputStreamSafe, AudioStream, AudioStreamBuilder, AudioStreamSafe,
    DataCallbackResult, PerformanceMode, SharingMode, Stereo, StreamState,
};
#[cfg(target_os = "android")]
use ringbuf::traits::Consumer;
use ringbuf::HeapCons;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
#[cfg(target_os = "android")]
use std::sync::Mutex;
use tracing::info;
#[cfg(target_os = "android")]
use tracing::{error, warn};

pub struct OboeAudioPlayback {
    running: Arc<AtomicBool>,
}

#[cfg(not(target_os = "android"))]
impl OboeAudioPlayback {
    pub fn start(_consumer: HeapCons<f32>) -> Result<Self, anyhow::Error> {
        info!("OboeAudioPlayback: Stub mode on non-Android platform");
        Ok(Self { running: Arc::new(AtomicBool::new(true)) })
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(target_os = "android")]
pub struct OboeCallback {
    consumer: Arc<Mutex<HeapCons<f32>>>,
    running: Arc<AtomicBool>,
    disconnected: Arc<AtomicBool>,
}

#[cfg(target_os = "android")]
impl AudioOutputCallback for OboeCallback {
    type FrameType = (f32, Stereo);

    fn on_audio_ready(
        &mut self,
        _audio_stream: &mut dyn AudioOutputStreamSafe,
        frames: &mut [(f32, f32)],
    ) -> DataCallbackResult {
        if !self.running.load(Ordering::Relaxed) || self.disconnected.load(Ordering::Relaxed) {
            return DataCallbackResult::Stop;
        }

        if let Ok(mut cons) = self.consumer.lock() {
            for frame in frames.iter_mut() {
                let left = cons.try_pop().unwrap_or(0.0);
                let right = cons.try_pop().unwrap_or(left);
                *frame = (left, right);
            }
        } else {
            for frame in frames.iter_mut() {
                *frame = (0.0, 0.0);
            }
        }

        DataCallbackResult::Continue
    }

    fn on_error_after_close(
        &mut self,
        _audio_stream: &mut dyn AudioOutputStreamSafe,
        error: oboe::Error,
    ) {
        warn!(
            "Oboe AAudio hardware routing change/disconnect: {:?}. Triggering automatic reconnect...",
            error
        );
        self.disconnected.store(true, Ordering::SeqCst);
    }
}

#[cfg(target_os = "android")]
impl OboeAudioPlayback {
    pub fn start(consumer: HeapCons<f32>) -> Result<Self, anyhow::Error> {
        let running = Arc::new(AtomicBool::new(true));
        let running_thread = Arc::clone(&running);
        let shared_consumer = Arc::new(Mutex::new(consumer));

        std::thread::spawn(move || {
            info!("Oboe AAudio supervisor thread active (hot-plug & cable-swap recovery enabled)");

            while running_thread.load(Ordering::Relaxed) {
                let disconnected = Arc::new(AtomicBool::new(false));
                let callback = OboeCallback {
                    consumer: Arc::clone(&shared_consumer),
                    running: Arc::clone(&running_thread),
                    disconnected: Arc::clone(&disconnected),
                };

                let stream_res = AudioStreamBuilder::default()
                    .set_format::<f32>()
                    .set_channel_count::<Stereo>()
                    .set_sample_rate(48000)
                    .set_performance_mode(PerformanceMode::LowLatency)
                    .set_sharing_mode(SharingMode::Shared)
                    .set_callback(callback)
                    .open_stream();

                match stream_res {
                    Ok(mut stream) => {
                        if let Err(e) = stream.start() {
                            error!("Failed to start Oboe AAudio stream: {:?}", e);
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            continue;
                        }

                        info!("Oboe AAudio stream running (P2 3.5mm Headphone / Edifier Line-Out)");

                        while running_thread.load(Ordering::Relaxed)
                            && !disconnected.load(Ordering::Relaxed)
                        {
                            match stream.get_state() {
                                StreamState::Started | StreamState::Starting => {
                                    std::thread::sleep(std::time::Duration::from_millis(200));
                                }
                                StreamState::Disconnected
                                | StreamState::Closing
                                | StreamState::Closed => {
                                    warn!("Oboe AAudio stream disconnected due to jack change. Reopening...");
                                    break;
                                }
                                _ => {
                                    std::thread::sleep(std::time::Duration::from_millis(200));
                                }
                            }
                        }

                        let _ = stream.stop();
                        let _ = stream.close();
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        error!("Failed to open Oboe stream: {:?}. Retrying in 300ms...", e);
                        std::thread::sleep(std::time::Duration::from_millis(300));
                    }
                }
            }

            info!("Oboe AAudio supervisor thread terminated");
        });

        Ok(Self { running })
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::traits::Split;

    #[test]
    fn test_oboe_playback_stub_initialization() {
        let ring_buffer = ringbuf::HeapRb::<f32>::new(1024);
        let (_prod, cons) = ring_buffer.split();
        let playback = OboeAudioPlayback::start(cons);
        assert!(playback.is_ok());
    }
}
