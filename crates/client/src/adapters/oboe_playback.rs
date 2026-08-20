#[cfg(target_os = "android")]
use ringbuf::traits::Consumer;
use ringbuf::HeapCons;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::info;

#[cfg(target_os = "android")]
use oboe::{
    AudioOutputCallback, AudioOutputStream, AudioStream, AudioStreamAsync, AudioStreamBuilder,
    ChannelCount, DataCallbackResult, PerformanceMode, SharingMode,
};

pub struct OboeAudioPlayback {
    running: Arc<AtomicBool>,
    #[cfg(target_os = "android")]
    _stream: AudioStreamAsync<AudioOutputCallback, f32>,
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
struct OboeCallback {
    consumer: HeapCons<f32>,
    running: Arc<AtomicBool>,
}

#[cfg(target_os = "android")]
impl AudioOutputCallback for OboeCallback {
    type FrameType = (f32, f32);

    fn on_audio_ready(
        &mut self,
        _audio_stream: &mut dyn AudioOutputStreamSafe,
        frames: &mut [Self::FrameType],
    ) -> DataCallbackResult {
        if !self.running.load(Ordering::Relaxed) {
            return DataCallbackResult::Stop;
        }

        for frame in frames.iter_mut() {
            let left = self.consumer.try_pop().unwrap_or(0.0);
            let right = self.consumer.try_pop().unwrap_or(left);
            *frame = (left, right);
        }

        DataCallbackResult::Continue
    }
}

#[cfg(target_os = "android")]
impl OboeAudioPlayback {
    pub fn start(consumer: HeapCons<f32>) -> Result<Self, anyhow::Error> {
        let running = Arc::new(AtomicBool::new(true));
        let callback = OboeCallback { consumer, running: Arc::clone(&running) };

        let stream = AudioStreamBuilder::default()
            .set_channel_count(ChannelCount::Stereo)
            .set_sample_rate(48000)
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sharing_mode(SharingMode::Exclusive)
            .set_callback(callback)
            .open_stream()?;

        info!("Oboe AAudio stream opened successfully on Android (P2/Headphone output)");

        Ok(Self { running, _stream: stream })
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
