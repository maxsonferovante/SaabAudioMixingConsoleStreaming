use crate::error::CoreError;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

impl AudioBuffer {
    pub fn new(samples: Vec<f32>, channels: u16, sample_rate: u32) -> Result<Self, CoreError> {
        if channels == 0 {
            return Err(CoreError::InvalidAudioBuffer("Channels must be > 0".into()));
        }
        if sample_rate == 0 {
            return Err(CoreError::InvalidAudioBuffer("Sample rate must be > 0".into()));
        }
        if samples.len() % (channels as usize) != 0 {
            return Err(CoreError::InvalidAudioBuffer(
                "Sample count is not aligned with channel count".into(),
            ));
        }

        Ok(Self { samples, channels, sample_rate })
    }

    pub fn silence(frame_count: usize, channels: u16, sample_rate: u32) -> Self {
        Self { samples: vec![0.0; frame_count * channels as usize], channels, sample_rate }
    }

    #[inline]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    #[inline]
    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    #[inline]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    pub fn frame_count(&self) -> usize {
        self.samples.len() / (self.channels as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_audio_buffer() {
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        let buf = AudioBuffer::new(samples.clone(), 2, 48000).expect("valid buffer");
        assert_eq!(buf.channels(), 2);
        assert_eq!(buf.sample_rate(), 48000);
        assert_eq!(buf.frame_count(), 2);
        assert_eq!(buf.samples(), &samples[..]);
    }

    #[test]
    fn test_unaligned_channels_fails() {
        let samples = vec![0.1, 0.2, 0.3];
        let result = AudioBuffer::new(samples, 2, 48000);
        assert!(result.is_err());
    }
}
