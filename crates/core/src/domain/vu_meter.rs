use super::audio_buffer::AudioBuffer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VuMeterReading {
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
    pub is_clipping: bool,
}

impl VuMeterReading {
    pub const ZERO: Self =
        Self { peak_left: 0.0, peak_right: 0.0, rms_left: 0.0, rms_right: 0.0, is_clipping: false };

    pub fn compute_from_buffer(buffer: &AudioBuffer) -> Self {
        let samples = buffer.samples();
        let channels = buffer.channels() as usize;
        let frame_count = buffer.frame_count();

        if frame_count == 0 || channels == 0 {
            return Self::ZERO;
        }

        let mut peak_l = 0.0f32;
        let mut peak_r = 0.0f32;
        let mut sum_sq_l = 0.0f32;
        let mut sum_sq_r = 0.0f32;

        if channels == 1 {
            for &sample in samples {
                let abs = sample.abs();
                if abs > peak_l {
                    peak_l = abs;
                }
                sum_sq_l += sample * sample;
            }
            peak_r = peak_l;
            sum_sq_r = sum_sq_l;
        } else {
            for i in 0..frame_count {
                let l = samples[i * channels];
                let r = samples[i * channels + 1];

                let abs_l = l.abs();
                if abs_l > peak_l {
                    peak_l = abs_l;
                }
                sum_sq_l += l * l;

                let abs_r = r.abs();
                if abs_r > peak_r {
                    peak_r = abs_r;
                }
                sum_sq_r += r * r;
            }
        }

        let rms_l = (sum_sq_l / frame_count as f32).sqrt();
        let rms_r = (sum_sq_r / frame_count as f32).sqrt();
        let is_clipping = peak_l >= 0.995 || peak_r >= 0.995;

        Self {
            peak_left: peak_l.min(1.0),
            peak_right: peak_r.min(1.0),
            rms_left: rms_l.min(1.0),
            rms_right: rms_r.min(1.0),
            is_clipping,
        }
    }
}

/// Smooth VU Meter processor with peak hold and exponential release decay
#[derive(Debug, Clone)]
pub struct VuMeterState {
    decay_factor: f32,
    hold_peak_left: f32,
    hold_peak_right: f32,
}

impl Default for VuMeterState {
    fn default() -> Self {
        Self::new(0.92)
    }
}

impl VuMeterState {
    pub fn new(decay_factor: f32) -> Self {
        Self {
            decay_factor: decay_factor.clamp(0.5, 0.99),
            hold_peak_left: 0.0,
            hold_peak_right: 0.0,
        }
    }

    pub fn process_reading(&mut self, instant: VuMeterReading) -> VuMeterReading {
        self.hold_peak_left = (self.hold_peak_left * self.decay_factor).max(instant.peak_left);
        self.hold_peak_right = (self.hold_peak_right * self.decay_factor).max(instant.peak_right);

        VuMeterReading {
            peak_left: self.hold_peak_left,
            peak_right: self.hold_peak_right,
            rms_left: instant.rms_left,
            rms_right: instant.rms_right,
            is_clipping: instant.is_clipping,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_vu_meter_sine_block() {
        let mut samples = Vec::new();
        for i in 0..100 {
            let val = (i as f32 * 0.1).sin();
            samples.push(val);
            samples.push(val);
        }
        let buf = AudioBuffer::new(samples, 2, 48000).expect("valid buffer");
        let meter = VuMeterReading::compute_from_buffer(&buf);
        assert!(meter.peak_left > 0.8 && meter.peak_left <= 1.0);
        assert!(meter.rms_left > 0.5);
    }

    #[test]
    fn test_vu_meter_state_decay() {
        let mut state = VuMeterState::new(0.9);
        let reading = VuMeterReading {
            peak_left: 1.0,
            peak_right: 1.0,
            rms_left: 0.7,
            rms_right: 0.7,
            is_clipping: true,
        };

        let first = state.process_reading(reading);
        assert_eq!(first.peak_left, 1.0);

        let quiet = VuMeterReading::ZERO;
        let second = state.process_reading(quiet);
        assert!((second.peak_left - 0.9).abs() < 0.01);
    }
}
