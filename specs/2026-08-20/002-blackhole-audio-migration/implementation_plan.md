# Migration Plan: Apple ScreenCaptureKit to BlackHole CoreAudio Virtual Driver

Migrate the macOS audio capture subsystem from Apple ScreenCaptureKit to **BlackHole** (an open-source CoreAudio virtual audio driver). This eliminates macOS screen recording system indicators, removes unnecessary GPU/CPU rendering overhead, enables bit-exact capture of master system audio with zero driver latency, supports high sample rates (44.1kHz, 48kHz, 96kHz, 192kHz), and routes audio exclusively to the Android P2 speaker output.

---

## User Review Required

> [!IMPORTANT]
> **BlackHole Installation Requirement on macOS**
> The server requires at least one BlackHole driver variant installed on macOS.
> - **Project Standard (Default)**: `brew install blackhole-16ch`
> - **Alternatives (Fully Compatible)**: `brew install blackhole-2ch` or `brew install blackhole-64ch`
> 
> When the server starts, it will automatically scan and bind to `BlackHole 16ch` by default (or available variants). If none is found, it will log step-by-step Homebrew installation instructions and fall back to the default audio input.

> [!NOTE]
> **Removal of Swift Linking & ScreenCaptureKit**
> - The `screencapturekit` crate and the custom `build.rs` linking macOS Swift runtime libraries will be completely removed.
> - Audio capture will run in 100% pure, safe Rust over Apple's native CoreAudio HAL via `cpal`.

---

## BlackHole Variants Comparison & Channel Guide

The acronym **"ch"** stands for independent **Audio Channels**. All variants execute the identical low-level macOS virtual driver kernel codebase with **zero additional driver latency**, but are scaled for different production workflows:

| Version | Channel Count | Production Use Case | Recommended for Project? |
| :--- | :--- | :--- | :--- |
| **`BlackHole 16ch`** | **16 Independent Channels** | Music production in DAWs (Logic Pro, Ableton, Reaper), advanced OBS multitrack routing, distinct sub-mixes (Game, Discord, Music), and 5.1/7.1 surround audio feeds. | **PROJECT STANDARD (Recommended)** |
| **`BlackHole 2ch`** | **2 Channels (Stereo: Left & Right)** | Standard stereo playback, Spotify, YouTube, common gaming, voice calls, and basic streaming. | Full Compatibility (Auto-Discovery) |
| **`BlackHole 64ch` / `256ch`** | **64 / 256 Independent Channels** | Large-scale recording studios, multi-instrument orchestral tracking, and complex industrial audio arrays. | Full Compatibility (Auto-Discovery) |

---

## Architecture & Design Decisions

1. **Pure CoreAudio HAL Capture (`crates/server`)**:
   - Interacts with BlackHole via CoreAudio with zero additional driver latency.
   - Eliminates all ScreenCaptureKit permissions, screen icons, and video compositor overhead.
2. **Intelligent Device Discovery**:
   - Auto-detects `BlackHole 2ch`, `BlackHole 16ch`, or `BlackHole 64ch`.
   - Supports CLI override (`cargo run --bin server -- --device "BlackHole 2ch"`) and `AUDIO_DEVICE` environment variable.
   - If not installed, outputs informative diagnostic logs with exact `brew install` commands.
3. **Dynamic Sample Rate & Multi-Channel Pipeline**:
   - Automatically adopts the sample rate configured on BlackHole (44.1kHz, 48kHz, 96kHz, 192kHz).
   - Encodes native sample rate into the 28-byte `AudioPacketHeader`.
   - Android client dynamically matches its Oboe AAudio stream sample rate or handles transparent playback.
4. **Broadcast Standard Channel Downmixing**:
   - **BlackHole 2ch**: Direct bit-exact stereo passthrough (L/R) with zero CPU overhead.
   - **BlackHole 16ch / 64ch**: Standard ITU-R BS.775 broadcast downmix ($L = L + 0.707 \cdot C + 0.707 \cdot Ls$, $R = R + 0.707 \cdot C + 0.707 \cdot Rs$) to preserve center dialogue and surround effects for 5.1/7.1 content.

---

## Proposed Changes

### Backend Server Layer (`crates/server`)

#### [MODIFY] [Cargo.toml](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/server/Cargo.toml)
- Remove `screencapturekit` dependency and target-conditional macOS dependencies.

#### [DELETE] [build.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/server/build.rs)
- Remove Swift runtime linking build script.

#### [MODIFY] [capture_macos.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/server/src/adapters/capture_macos.rs)
- Refactor `MacAudioCapture` to implement BlackHole auto-discovery (`BlackHole 2ch`, `BlackHole 16ch`, `BlackHole 64ch`).
- Implement dynamic sample rate extraction from the audio device configuration.
- Implement ITU-R BS.775 downmix for multi-channel frames.
- Add clear installation guidance when BlackHole is not detected.

#### [MODIFY] [main.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/server/src/main.rs)
- Update startup banner and CLI parameter handling for BlackHole device selection.

---

### Client Layer (`crates/client`)

#### [MODIFY] [udp_receiver.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/client/src/adapters/udp_receiver.rs)
- Verify packet unpacking with dynamic sample rates (44.1kHz to 192kHz).

#### [MODIFY] [oboe_playback.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/client/src/adapters/oboe_playback.rs)
- Configure Oboe AAudio builder for dynamic/unspecified sample rate to let the Android audio HAL pick the optimal native DAC hardware sample rate.

---

### Documentation

#### [MODIFY] [README.md](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/README.md)
- Replace `screencapturekit` with `BlackHole` in the Core Libraries table, architecture overview, and prerequisites.

---

## Verification Plan

### Automated Tests
```bash
# Run all workspace unit and integration tests
cargo test --workspace

# Run strict clippy linter
cargo clippy --workspace -- -D warnings

# Verify formatting
cargo fmt --check
```

### Manual Verification
1. **Device Discovery Test**:
   - Run `cargo run --bin server` without BlackHole to verify helpful guidance output.
   - Run `cargo run --bin server` with `BlackHole 2ch` to verify automatic device selection, sample rate detection, and streaming.
2. **Audio Streaming & Quality Test**:
   - Set macOS output to `BlackHole 2ch` in System Settings -> Sound.
   - Play audio in Spotify and verify that audio is streamed directly to the Android device without triggering any screen recording icons or playing out of the Mac speaker.
