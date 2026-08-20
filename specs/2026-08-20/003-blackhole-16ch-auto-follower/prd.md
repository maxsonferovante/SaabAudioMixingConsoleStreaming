# PRD 003: BlackHole 16ch Dynamic Auto-Follower & Multi-Channel Downmix Engine

## Problem Statement

When using macOS audio streaming with the BlackHole CoreAudio virtual driver, users frequently switch between multi-channel virtual drivers such as **BlackHole 16ch** (for DAWs, surround content, and multi-track production) and **BlackHole 2ch** (for standard stereo media playback). 

Currently, the server binds to a fixed audio device at startup. When a user switches their macOS Default Sound Output to **BlackHole 16ch** while the server is running, the server remains attached to the previous audio handle, causing complete silence on the mobile receiver. Additionally, audio streams on multi-channel drivers may route signal across different channel pairs (e.g. channels 1/2, 3/4, or surround fold-down), which can lead to dropped audio or digital clipping when multiple DAW buses play concurrently without a soft-limiting downmixing pipeline.

## Solution

Implement an intelligent, real-time **CoreAudio Dynamic Auto-Follower & 16-Channel Downmix Engine** in the macOS server backend:

1. **Dynamic CoreAudio Auto-Follower**: A supervisor loop in the capture adapter that continuously monitors macOS active sound output and automatically rebinds the CoreAudio HAL input stream in under 100ms whenever the user changes their output device in macOS System Settings.
2. **Smart 16-Channel Downmixer with Anti-Clipping Soft Limiter**: A mathematical downmixing algorithm that blends all active channel pairs (even channels to Left, odd channels to Right) with ITU-R BS.775 acoustic power conservation and a hyperbolic tangent ($\tanh$) soft limiter to prevent digital distortion.
3. **16-Channel Real-Time Diagnostics**: Granular telemetry logging that reports signal energy across all channel pairs, enabling instant diagnosis of DAW and application routing configurations.

---

## User Stories

1. As a live audio engineer, I want the server to automatically detect when I switch macOS sound output to BlackHole 16ch, so that I do not have to restart the streaming server manually.
2. As a DAW user (Logic Pro / Ableton / Reaper), I want multi-channel audio routed to any of the 16 channels in BlackHole 16ch to be automatically mixed down to stereo, so that I can monitor complex sessions on my mobile device.
3. As a music producer, I want simultaneous multi-channel audio playback to pass through an anti-clipping soft limiter ($\tanh$), so that heavy multi-track summing does not produce harsh digital distortion in my speakers.
4. As a user switching between YouTube (stereo) and a DAW (16 channels), I want seamless transitions between BlackHole 2ch and BlackHole 16ch with sub-100ms recovery, so that my audio stream never drops out.
5. As a streamer, I want the server logs to display an active channel map (e.g., Ch 0-1, Ch 2-3, Ch 4-5 signal levels), so that I can immediately identify which channel pair my applications are routing sound to.
6. As an audio creator, I want center channel and surround signals on BlackHole 16ch to follow ITU-R BS.775 acoustic power conservation ($-3\text{dB}$ attenuation), so that downmixed vocals and effects maintain natural perceived loudness.
7. As a system administrator, I want the auto-follower to gracefully handle unplugged or uninstalled drivers without panicking, falling back to available virtual or hardware inputs.
8. As a developer, I want comprehensive unit test coverage for 16-channel downmixing, channel-pair mapping, and soft-limiter linearity, so that regressions in audio quality are caught before release.

---

## Implementation Decisions

### Dynamic Device Auto-Follower Architecture
- **Supervisor Loop**: The capture subsystem runs a lightweight monitoring loop that queries `host.default_output_device()` periodically and verifies whether the active output device matches the currently open CPAL input stream.
- **Hot-Migration**: When a mismatch is detected (e.g. output switched from `BlackHole 2ch` to `BlackHole 16ch`), the existing stream is closed and a new stream configured for the target device's channel count and sample rate is opened in under 100ms.

### Multi-Channel Downmix Algorithm & Anti-Clipping
- **Channel Pairing**:
  - Even channels ($0, 2, 4, 6, \dots, 14$) are folded into the **Left** channel.
  - Odd channels ($1, 3, 5, 7, \dots, 15$) are folded into the **Right** channel.
- **Acoustic Power Weighting**: Channels beyond the primary stereo bus ($0$ and $1$) are scaled by $\frac{1}{\sqrt{2}} \approx 0.70710678$ to maintain energy conservation.
- **Hyperbolic Tangent Soft Limiter**:
  $$y = \tanh(x)$$
  Provides linear response ($y \approx x$) for standard audio levels ($|x| < 0.7$) while smoothly saturating peaks as $|x| \to 1.0$, completely eliminating hard digital clipping wraps.

### 16-Channel Diagnostic Telemetry
- The capture callback computes Peak and RMS amplitudes per channel pair over accumulated 5ms frame blocks.
- Telemetry outputs structured logs displaying per-pair signal activity during streaming.

---

## Testing Decisions

- **Domain Mathematical Invariance Tests**: Unit tests verifying that stereo passthrough preserves bit-exact values, center channels receive $-3\text{dB}$ attenuation, and high-amplitude multi-track sums are softly clamped below $1.0$.
- **Multi-Channel Mapping Verification**: Tests simulating frames with signal solely on secondary channels (e.g., channels 2/3 or channels 8/9) to ensure they are properly routed to Left and Right outputs.
- **Zero-I/O In-Memory Verification**: Pure algorithmic validation executing in microseconds without hardware device dependencies.

---

## Out of Scope

- Multi-track individual fader control per channel on the Android UI (mixing is downmixed to master stereo for this milestone).
- D-PDU network audio routing over WAN/Internet.
- DSP equalization / multi-band dynamics on individual BlackHole channel inputs.

---

## Further Notes

- **Driver Compatibility**: Compatible with BlackHole 2ch, BlackHole 16ch, and BlackHole 64ch on macOS 13 (Ventura), macOS 14 (Sonoma), and macOS 15 (Sequoia).
- **CoreAudio Permissions**: Requires Terminal/IDE Microphone privacy permissions enabled in macOS System Settings.
