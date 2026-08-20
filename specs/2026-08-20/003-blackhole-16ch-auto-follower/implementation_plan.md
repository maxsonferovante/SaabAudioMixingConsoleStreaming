# Implementation Plan: BlackHole 16ch Dynamic Auto-Follower & Multi-Channel Downmix Engine

## Problem Statement

When users install and switch between **BlackHole 16ch** and **BlackHole 2ch** in macOS *System Settings -> Sound -> Output*, the current audio capture server implementation:
1. Locks onto a single audio device on server startup and fails to track runtime macOS output device changes, causing silence when switching between 2ch and 16ch.
2. Only checks primary channels (0 and 1) or basic center fold-down, missing secondary or alternative channel pairs routed by multi-channel applications or DAWs.
3. Lacks granular channel-level telemetry to inspect which of the 16 channels are carrying active audio signal.

---

## User Review Required

> [!IMPORTANT]
> - The server will now run a **Dynamic Auto-Follower Watchdog** in a background supervisor thread. If you switch your Mac output between `BlackHole 16ch`, `BlackHole 2ch`, or `BlackHole 64ch` in System Settings while streaming, the server will detect the change and migrate the capture stream in under 100ms with zero downtime.
> - The 16-channel downmix engine will sum all active channel pairs (Evens $\to$ Left, Odds $\to$ Right) with acoustic $-3\text{dB}$ scaling and hyperbolic tangent soft-limiting ($\tanh$) to prevent digital clipping when multiple DAW buses or media streams play simultaneously.

---

## Proposed Changes

### Backend Server (`crates/server`)

#### [MODIFY] [`crates/server/src/adapters/capture_macos.rs`](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/server/src/adapters/capture_macos.rs)
- **Dynamic Device Auto-Follower**: Implement a supervisor loop that continuously queries `host.default_output_device()` and compares it with the currently active capture stream. When a change is detected (e.g. user selected `BlackHole 16ch` in Sound Settings), rebind the CPAL input stream seamlessly.
- **Smart 16-Channel Downmixer with Anti-Clipping Soft Limiter**:
  - Map Even channels ($0, 2, 4, 6, \dots, 14$) to Left and Odd channels ($1, 3, 5, 7, \dots, 15$) to Right.
  - Apply ITU-R BS.775 power conservation ($C \times \frac{1}{\sqrt{2}}$) and soft clipping:
    $$y = \tanh(x)$$
    ensuring linear passthrough for normal levels ($|x| < 0.7$) and smooth saturation without harsh digital wrapping above $0.99$.
- **16-Channel Diagnostic Telemetry**:
  - Calculate RMS/Peak energy per channel pair ($0\text{-}1, 2\text{-}3, 4\text{-}5, \dots, 14\text{-}15$).
  - When signal is detected, display active channel maps in log output (e.g., `Ch 0-1: 0.724 | Ch 2-3: 0.000 | Ch 4-5: 0.120`).

#### [MODIFY] [`crates/server/src/adapters/capture_macos.rs` Unit Tests](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/server/src/adapters/capture_macos.rs)
- Add unit tests for 16-channel downmixing with signals on channels 0-1, secondary routing on channels 2-3, surround routing, and full 16-channel simultaneous playback with soft limiter validation.

---

## Verification Plan

### Automated Tests
- Run workspace test suite:
  ```bash
  cargo test --workspace
  ```
- Run strict Clippy and formatting checks:
  ```bash
  cargo clippy --workspace -- -D warnings
  cargo fmt --check
  ```

### Manual Verification
1. Start the server on macOS:
   ```bash
   cargo run --bin server -- 192.168.15.5:48480
   ```
2. Open **System Settings -> Sound -> Output** and switch from **BlackHole 2ch** to **BlackHole 16ch**.
3. Verify that the server logs report:
   - Output device migration: `Auto-Follower: Switched to active macOS sound output: BlackHole 16ch`
   - Active channel telemetry: `BlackHole 16ch Capture: peak signal: 0.XXXX [Ch 0-1: 0.XXXX, Ch 2-3: ...]`
4. Play audio on YouTube / Spotify and verify clean stereo sound output on the Android client and connected Edifier speaker.
