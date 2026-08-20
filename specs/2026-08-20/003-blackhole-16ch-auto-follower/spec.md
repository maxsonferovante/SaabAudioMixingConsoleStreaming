# SPEC 003: BlackHole 16ch Dynamic Auto-Follower & Multi-Channel Downmix Engine

## Problem Statement

Users operating macOS audio production setups frequently alternate between multi-channel audio drivers (such as **BlackHole 16ch** for DAWs, surround routing, and multi-track workflows) and standard stereo virtual drivers (**BlackHole 2ch** for Spotify, YouTube, and browser playback). 

In the initial implementation, the audio capture subsystem statically bound to an audio input device at startup. When a user changed their macOS Default Sound Output to **BlackHole 16ch** in System Settings while the server was running, the server continued reading from the now-inactive previous audio handle, causing complete silence on the remote stream. Furthermore, audio signals routed across non-standard channel pairs (e.g., DAW channels 3/4, 5/6, or 5.1 surround) were susceptible to signal dropping or digital clipping without a comprehensive multi-channel downmixer equipped with soft-limiting.

## Solution

Implement an automated, low-latency **CoreAudio Dynamic Auto-Follower and 16-Channel Downmixing Engine** within the macOS server capture adapter (`crates/server`):

1. **Dynamic CoreAudio Device Auto-Follower**: A dedicated background supervisor thread that continuously samples the active macOS system output device (`host.default_output_device()`). Upon detecting an output device switch (e.g. `BlackHole 2ch` $\to$ `BlackHole 16ch`), it tears down the existing stream and rebinds the CoreAudio HAL input stream to the new device in under 100ms without process interruption.
2. **Smart 16-Channel Downmixer with Anti-Clipping Soft Limiter**: A mathematical downmixing algorithm that folds all active multi-channel pairs (Even channels $\to$ Left, Odd channels $\to$ Right) with ITU-R BS.775 power scaling ($-3\text{dB}$) and a hyperbolic tangent ($\tanh$) soft limiter to guarantee zero digital clipping.
3. **16-Channel Granular Telemetry**: Real-time channel-pair energy monitoring to provide instant diagnostics on which DAW channels are carrying active audio signal.

---

## User Stories

1. As a sound engineer, I want the server to automatically detect when I switch macOS sound output to BlackHole 16ch, so that I do not have to restart the streaming server manually.
2. As a DAW user (Logic Pro / Ableton / Reaper), I want multi-channel audio routed to any of the 16 channels in BlackHole 16ch to be automatically mixed down to stereo, so that I can monitor complex sessions on my mobile device.
3. As a music producer, I want simultaneous multi-channel audio playback to pass through an anti-clipping soft limiter ($\tanh$), so that heavy multi-track summing does not produce harsh digital distortion in my speakers.
4. As a user switching between YouTube (stereo) and a DAW (16 channels), I want seamless transitions between BlackHole 2ch and BlackHole 16ch with sub-100ms recovery, so that my audio stream never drops out.
5. As a streamer, I want the server logs to display an active channel map (e.g., Ch 0-1, Ch 2-3, Ch 4-5 signal levels), so that I can immediately identify which channel pair my applications are routing sound to.
6. As an audio creator, I want center channel and surround signals on BlackHole 16ch to follow ITU-R BS.775 acoustic power conservation ($-3\text{dB}$ attenuation), so that downmixed vocals and effects maintain natural perceived loudness.
7. As a system administrator, I want the auto-follower to gracefully handle unplugged or uninstalled drivers without panicking, falling back to available virtual or hardware inputs.
8. As a developer, I want comprehensive unit test coverage for 16-channel downmixing, channel-pair mapping, and soft-limiter linearity, so that regressions in audio quality are caught before release.

---

## Implementation Decisions

### Hexagonal Architecture & Seam Definitions
- **Secondary Adapter (`MacAudioCapture`)**: Implements `AudioCapturePort` in `crates/server`.
- **Dynamic Auto-Follower Supervisor Thread**:
  - Operates concurrently with Tokio runtime.
  - Checks `host.default_output_device()` on a 250ms cadence.
  - Compares the active output device name against the current CPAL input stream device name.
  - When a transition is detected, it cleanly stops the old stream, acquires the new device configuration (`channels`, `sample_rate`), and spawns a new CPAL input stream.

```
+-------------------------------------------------------------+
|               MacAudioCapture Supervisor Thread             |
|                                                             |
|   1. Query host.default_output_device() (every 250ms)        |
|   2. If device != current_device:                           |
|        - Stop & drop active cpal::Stream                    |
|        - Open new cpal::Stream on target device             |
|        - Log migration event with channel count             |
+------------------------------+------------------------------+
                               |
                               v
+-------------------------------------------------------------+
|              CPAL Input Callback (data: &[f32])             |
|                                                             |
|   - Multi-channel frame chunking (frame = 16 samples)       |
|   - downmix_frame_to_stereo(frame)                          |
|       - Evens -> Left, Odds -> Right                        |
|       - ITU-R BS.775 power scaling (FRAC_1_SQRT_2)          |
|       - Soft limiting: y = tanh(x)                          |
|   - Accumulate into 5ms AudioBuffer (48kHz: 240 frames)     |
|   - Invoke shared AudioCapturePort callback                 |
+-------------------------------------------------------------+
```

### Multi-Channel Downmix Algorithm Prototype Shape
```rust
pub fn downmix_frame_to_stereo(frame: &[f32]) -> (f32, f32) {
    match frame.len() {
        0 => (0.0, 0.0),
        1 => (frame[0], frame[0]),
        2 => (frame[0], frame[1]),
        6 => {
            // 5.1 Surround Downmix (ITU-R BS.775)
            let c = frame[2] * FRAC_1_SQRT_2;
            let l = (frame[0] + c + frame[4] * FRAC_1_SQRT_2).tanh();
            let r = (frame[1] + c + frame[5] * FRAC_1_SQRT_2).tanh();
            (l, r)
        }
        _ => {
            // 16-channel and arbitrary multi-channel summing
            let mut l = frame[0];
            let mut r = frame.get(1).copied().unwrap_or(0.0);

            // Fold in center (Ch 2) if active
            if let Some(&c) = frame.get(2) {
                if c.abs() > 0.0001 {
                    l += c * FRAC_1_SQRT_2;
                    r += c * FRAC_1_SQRT_2;
                }
            }

            // Sum additional active stereo pairs (Ch 4-5, 6-7, 8-9, ..., 14-15)
            let mut i = 4;
            while i < frame.len() {
                let pair_l = frame[i];
                let pair_r = if i + 1 < frame.len() { frame[i + 1] } else { pair_l };
                if pair_l.abs() > 0.0001 || pair_r.abs() > 0.0001 {
                    l += pair_l * FRAC_1_SQRT_2;
                    r += pair_r * FRAC_1_SQRT_2;
                }
                i += 2;
            }

            // If main bus (0 & 1) is silent but secondary bus (2 & 3) is active
            if frame[0].abs() <= 0.00001 && frame.get(1).copied().unwrap_or(0.0).abs() <= 0.00001 {
                if let (Some(&ch2), Some(&ch3)) = (frame.get(2), frame.get(3)) {
                    if ch2.abs() > 0.0001 || ch3.abs() > 0.0001 {
                        l = ch2;
                        r = ch3;
                    }
                }
            }

            (l.tanh(), r.tanh())
        }
    }
}
```

---

## Testing Decisions

- **Downmix Unit Tests**:
  - `test_downmix_stereo_passthrough`: Validates bit-exact passthrough for 2-channel audio.
  - `test_downmix_16_channel_secondary_bus`: Validates routing when sound is present on channels 2 & 3 or channels 4 & 5.
  - `test_downmix_16_channel_full_saturation`: Validates that simultaneous maximum signals on all 16 channels are smoothly limited by $\tanh$ below $1.0$.
  - `test_downmix_5_1_surround_itur_bs775`: Validates ITU-R BS.775 power conservation.
- **In-Memory Seam**: All algorithmic and DSP tests run in-memory with zero I/O or driver dependencies.

---

## Out of Scope

- Matrix GUI for custom channel patching (16x16 matrix mixer).
- Individual channel volume faders on client UI (global downmix delivered to master fader).
- Dynamic channel phase cancellation analyzers.

---

## Further Notes

- **Driver Installation**: BlackHole 16ch is installed via `brew install blackhole-16ch`.
- **macOS Security Permissions**: Terminal or IDE must have Microphone permission enabled in macOS System Settings.
