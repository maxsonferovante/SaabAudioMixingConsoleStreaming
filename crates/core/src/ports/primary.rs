use crate::domain::{AudioBuffer, DecibelVolume, MuteState};
use crate::error::CoreError;

pub trait ProcessAudioUseCase: Send + Sync {
    /// Processes an input audio buffer (DSP, volume scaling, anti-pop ramp, VU computation)
    /// and streams the processed pure/calibrated audio to secondary ports.
    fn process_block(&mut self, input: AudioBuffer) -> Result<AudioBuffer, CoreError>;
}

pub trait AdjustVolumeUseCase: Send + Sync {
    fn set_master_volume(&mut self, volume: DecibelVolume);
    fn get_master_volume(&self) -> DecibelVolume;
}

pub trait ToggleMuteUseCase: Send + Sync {
    fn set_mute(&mut self, muted: bool);
    fn set_dim(&mut self, dimmed: bool);
    fn get_state(&self) -> MuteState;
}
