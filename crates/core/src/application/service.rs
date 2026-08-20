use crate::domain::{AudioBuffer, DecibelVolume, GainRamp, MuteState, VuMeterReading};
use crate::error::CoreError;
use crate::ports::primary::{AdjustVolumeUseCase, ProcessAudioUseCase, ToggleMuteUseCase};
use crate::ports::secondary::{AudioStreamerPort, TelemetryBroadcasterPort};
use std::sync::Arc;

pub struct MixerService {
    volume: DecibelVolume,
    mute_state: MuteState,
    gain_ramp: GainRamp,
    sequence_number: u64,
    streamer: Option<Arc<dyn AudioStreamerPort>>,
    telemetry: Option<Arc<dyn TelemetryBroadcasterPort>>,
}

impl MixerService {
    pub fn new(
        streamer: Option<Arc<dyn AudioStreamerPort>>,
        telemetry: Option<Arc<dyn TelemetryBroadcasterPort>>,
    ) -> Self {
        Self {
            volume: DecibelVolume::UNITY,
            mute_state: MuteState::Unmuted,
            gain_ramp: GainRamp::new(1.0),
            sequence_number: 0,
            streamer,
            telemetry,
        }
    }

    fn calculate_effective_target_gain(&self) -> f32 {
        match self.mute_state {
            MuteState::Muted => 0.0,
            MuteState::Dimmed => {
                (self.volume.as_db() - 20.0).clamp(DecibelVolume::MIN_DB, DecibelVolume::MAX_DB)
            }
            MuteState::Unmuted => self.volume.to_linear_gain().as_f32(),
        }
    }
}

impl ProcessAudioUseCase for MixerService {
    fn process_block(&mut self, mut input: AudioBuffer) -> Result<AudioBuffer, CoreError> {
        let target_gain = self.calculate_effective_target_gain();
        let frame_count = input.frame_count();
        let channels = input.channels() as usize;

        // 5ms anti-pop smoothing window (240 samples at 48kHz)
        self.gain_ramp.set_target(target_gain, 240.min(frame_count));

        // Apply gain smoothing across samples
        let samples = input.samples_mut();
        for i in 0..frame_count {
            let gain = self.gain_ramp.next_gain();
            for ch in 0..channels {
                samples[i * channels + ch] *= gain;
            }
        }

        // Calculate VU Meter reading
        let vu = VuMeterReading::compute_from_buffer(&input);
        if let Some(ref tel) = self.telemetry {
            let _ = tel.broadcast_vu(&vu);
        }

        // Forward to Audio Streamer port if attached
        if let Some(ref streamer) = self.streamer {
            self.sequence_number = self.sequence_number.wrapping_add(1);
            let timestamp_us = 0; // Filled with real clock or passed in
            streamer.stream_audio(&input, self.sequence_number, timestamp_us)?;
        }

        Ok(input)
    }
}

impl AdjustVolumeUseCase for MixerService {
    fn set_master_volume(&mut self, volume: DecibelVolume) {
        self.volume = volume;
    }

    fn get_master_volume(&self) -> DecibelVolume {
        self.volume
    }
}

impl ToggleMuteUseCase for MixerService {
    fn set_mute(&mut self, muted: bool) {
        if muted {
            self.mute_state = MuteState::Muted;
        } else if self.mute_state == MuteState::Muted {
            self.mute_state = MuteState::Unmuted;
        }
    }

    fn set_dim(&mut self, dimmed: bool) {
        if dimmed {
            self.mute_state = MuteState::Dimmed;
        } else if self.mute_state == MuteState::Dimmed {
            self.mute_state = MuteState::Unmuted;
        }
    }

    fn get_state(&self) -> MuteState {
        self.mute_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockStreamer {
        stream_count: AtomicUsize,
    }

    impl AudioStreamerPort for MockStreamer {
        fn stream_audio(
            &self,
            _buffer: &AudioBuffer,
            _seq: u64,
            _ts: u64,
        ) -> Result<(), CoreError> {
            self.stream_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_mixer_service_process_and_stream() {
        let streamer = Arc::new(MockStreamer { stream_count: AtomicUsize::new(0) });
        let mut service = MixerService::new(Some(streamer.clone()), None);

        let input = AudioBuffer::new(vec![0.5, 0.5, 0.5, 0.5], 2, 48000).expect("valid buffer");
        let output = service.process_block(input).expect("process block");

        assert_eq!(output.frame_count(), 2);
        assert_eq!(streamer.stream_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_mixer_service_mute() {
        let mut service = MixerService::new(None, None);
        service.set_mute(true);
        assert_eq!(service.get_state(), MuteState::Muted);

        // Process audio after mute transition
        let input = AudioBuffer::new(vec![1.0; 480], 2, 48000).expect("valid buffer");
        let output = service.process_block(input).expect("process block");
        // Gain ramp attenuates to 0.0
        let last_sample = output.samples()[output.samples().len() - 1];
        assert_eq!(last_sample, 0.0);
    }
}
