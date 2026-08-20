#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecibelVolume(f32);

impl DecibelVolume {
    pub const MIN_DB: f32 = -80.0;
    pub const MAX_DB: f32 = 6.0;
    pub const UNITY: Self = Self(0.0);
    pub const SILENCE: Self = Self(Self::MIN_DB);

    pub fn new(db: f32) -> Self {
        Self(db.clamp(Self::MIN_DB, Self::MAX_DB))
    }

    pub fn as_db(&self) -> f32 {
        self.0
    }

    pub fn to_linear_gain(&self) -> LinearGain {
        if self.0 <= Self::MIN_DB {
            LinearGain(0.0)
        } else {
            let linear = 10.0f32.powf(self.0 / 20.0);
            LinearGain(linear.clamp(0.0, 2.0))
        }
    }
}

impl Default for DecibelVolume {
    fn default() -> Self {
        Self::UNITY
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearGain(f32);

impl LinearGain {
    pub const ZERO: Self = Self(0.0);
    pub const UNITY: Self = Self(1.0);

    pub fn new(gain: f32) -> Self {
        Self(gain.clamp(0.0, 2.0))
    }

    #[inline]
    pub fn as_f32(&self) -> f32 {
        self.0
    }

    pub fn to_decibels(&self) -> DecibelVolume {
        if self.0 <= 0.0001 {
            DecibelVolume::SILENCE
        } else {
            let db = 20.0 * self.0.log10();
            DecibelVolume::new(db)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuteState {
    Unmuted,
    Muted,
    Dimmed, // -20dB
}

/// Anti-pop gain smoother interpolating smoothly across buffers
#[derive(Debug, Clone)]
pub struct GainRamp {
    current_gain: f32,
    target_gain: f32,
    step: f32,
    remaining_frames: usize,
}

impl GainRamp {
    pub fn new(initial_gain: f32) -> Self {
        Self {
            current_gain: initial_gain,
            target_gain: initial_gain,
            step: 0.0,
            remaining_frames: 0,
        }
    }

    pub fn set_target(&mut self, target: f32, transition_frames: usize) {
        if (self.target_gain - target).abs() < 1e-6 {
            return;
        }
        self.target_gain = target;
        if transition_frames == 0 {
            self.current_gain = target;
            self.step = 0.0;
            self.remaining_frames = 0;
        } else {
            self.remaining_frames = transition_frames;
            self.step = (target - self.current_gain) / (transition_frames as f32);
        }
    }

    #[inline]
    pub fn next_gain(&mut self) -> f32 {
        if self.remaining_frames == 0 {
            self.current_gain = self.target_gain;
            return self.current_gain;
        }

        self.remaining_frames -= 1;
        if self.remaining_frames == 0 {
            self.current_gain = self.target_gain;
            self.step = 0.0;
        } else {
            self.current_gain += self.step;
        }
        self.current_gain
    }

    #[inline]
    pub fn current(&self) -> f32 {
        self.current_gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_db_to_linear() {
        assert_eq!(DecibelVolume::new(0.0).to_linear_gain().as_f32(), 1.0);
        assert_eq!(DecibelVolume::SILENCE.to_linear_gain().as_f32(), 0.0);
        let plus_6db = DecibelVolume::new(6.0).to_linear_gain().as_f32();
        assert!((plus_6db - 1.995).abs() < 0.01);
    }

    #[test]
    fn test_gain_ramp_smooth_transition() {
        let mut ramp = GainRamp::new(1.0);
        ramp.set_target(0.0, 100);
        for _ in 0..50 {
            ramp.next_gain();
        }
        assert!((ramp.current() - 0.5).abs() < 0.02);
        for _ in 0..60 {
            ramp.next_gain();
        }
        assert_eq!(ramp.current(), 0.0);
    }
}
