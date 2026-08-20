# PRD 002: BlackHole 16ch CoreAudio Virtual Driver Migration

## Problem Statement

The initial macOS audio capture implementation utilized Apple's `ScreenCaptureKit` (`SCStream`). While functional without virtual drivers, ScreenCaptureKit introduces severe operational trade-offs for studio audio streaming, mixing, and multitrack workflows:

1. **System Indicator & Security Prompts**: ScreenCaptureKit activates the macOS system-wide screen recording indicator (purple icon in the menu bar) and requires continuous screen recording permissions, causing visual distraction and security friction.
2. **Computational Overhead**: ScreenCaptureKit operates inside the macOS window server compositor, capturing display frames in addition to audio, which introduces unnecessary GPU and CPU rendering overhead.
3. **Sound Duplication / Dual Output**: Because ScreenCaptureKit captures system audio at the display output stage, audio plays simultaneously through the physical Mac speakers and the remote Android receiver device, defeating the purpose of routing audio exclusively to the external sound system.
4. **Channel & Routing Limitations**: ScreenCaptureKit locks audio capture to basic 2-channel stereo, preventing advanced studio workflows such as DAW multitrack routing (Logic Pro, Ableton, Reaper), multitrack broadcast streaming (OBS Studio), and 5.1/7.1 surround audio feeds.

## Solution

Migrate the macOS audio capture subsystem from `ScreenCaptureKit` to **BlackHole 16ch** as the primary default CoreAudio virtual audio driver ([github.com/existentialaudio/blackhole](https://github.com/existentialaudio/blackhole)):

1. **BlackHole 16ch Default Standard**: Adopt `BlackHole 16ch` as the primary standard driver for the system, enabling simultaneous routing of master system audio, discrete DAW channels, game audio, communications (Discord/Zoom), and surround streams.
2. **Pure CoreAudio HAL Integration**: Direct digital capture from BlackHole's zero-latency virtual audio loopback via `cpal` and macOS CoreAudio HAL, eliminating Swift runtime linking (`build.rs`) and the `screencapturekit` crate entirely.
3. **Intelligent Device Auto-Discovery**: Automatic discovery prioritizing `BlackHole 16ch` by default, with seamless compatibility for `BlackHole 2ch` and `BlackHole 64ch`, CLI/environment overrides, and step-by-step Homebrew installation instructions.
4. **High-Resolution Dynamic Sample Rates**: Native support for 44.1kHz, 48kHz, 88.2kHz, 96kHz, 176.4kHz, and 192kHz streams, propagating the exact sample rate to the Android receiver via binary datagram headers.
5. **Standard Broadcast Downmixing (ITU-R BS.775)**: Automatic ITU-R BS.775 weighted broadcast downmixing from 16-channel / surround feeds to clean stereo for the Android 3.5mm P2 audio jack.
6. **Exclusive Sound Routing**: Directing macOS system output to BlackHole isolates system audio entirely, transmitting it exclusively to the Android device and its 3.5mm P2 audio jack.

---

## BlackHole Variants Comparison & Channel Guide

The acronym **"ch"** stands for independent **Audio Channels**. All variants execute the identical low-level macOS virtual driver kernel codebase with **zero additional driver latency**, but are scaled for different production workflows:

| Version | Channel Count | Production Use Case | Recommended for Project? |
| :--- | :--- | :--- | :--- |
| **`BlackHole 16ch`** | **16 Independent Channels** | Music production in DAWs (Logic Pro, Ableton, Reaper), advanced OBS multitrack routing, distinct sub-mixes (Game, Discord, Music), and 5.1/7.1 surround audio feeds. | **PROJECT STANDARD (Recommended)** |
| **`BlackHole 2ch`** | **2 Channels (Stereo: Left & Right)** | Standard stereo playback, Spotify, YouTube, common gaming, voice calls, and basic streaming. | Full Compatibility (Auto-Discovery) |
| **`BlackHole 64ch` / `256ch`** | **64 / 256 Independent Channels** | Large-scale recording studios, multi-instrument orchestral tracking, and complex industrial audio arrays. | Full Compatibility (Auto-Discovery) |

### Technical Rationale for Adopting BlackHole 16ch as Standard
- **Multitrack & DAW Routing Flexibility**: Enables creators to assign discrete application audio buses to separate channels (e.g., Spotify on ch 1-2, Discord on ch 3-4, DAW on ch 5-8) while our server captures the aggregate mix.
- **Native Surround 5.1 & 7.1 Support**: Ingests multi-channel audio feeds from games and movies without channel truncation, applying intelligent ITU-R BS.775 downmixing to the smartphone's physical 3.5mm P2 stereo output.
- **Single Driver Package**: The user only needs to install **one** driver package on macOS via Homebrew (`brew install blackhole-16ch`).

---

## User Stories

1. As a macOS user, I want system audio captured via `BlackHole 16ch` without triggering the macOS screen recording menu bar icon, so that my desktop remains clean and unmonitored.
2. As a streamer, I want audio to play exclusively through the external speaker connected to my Android phone's 3.5mm P2 jack rather than echoing through the Mac speakers, so that my acoustic monitoring is isolated.
3. As a power user, I want the server to auto-discover `BlackHole 16ch` by default on launch, so that no manual configuration is required.
4. As a new user who has not yet installed BlackHole, I want the server to output copy-pasteable Homebrew commands (`brew install blackhole-16ch`), so that I can install the driver with a single terminal command.
5. As an audio professional, I want to stream high-resolution 96kHz or 192kHz audio from my 16-channel DAW session without downsampling, so that studio-grade audio fidelity is preserved.
6. As a gamer or movie viewer playing surround 5.1/7.1 content on `BlackHole 16ch`, I want automatic ITU-R BS.775 broadcast downmixing to stereo, so that center dialogue and rear surround effects remain balanced without phase cancellation.
7. As an Android user, I want the Google Oboe AAudio playback engine to automatically adapt to the incoming sample rate, so that audio playback is pitch-perfect without drift.
8. As a developer, I want the `crates/server` crate to compile in pure safe Rust without Swift runtime linking scripts, so that compilation is fast and portable.
9. As a developer, I want all workspace unit tests and integration tests to validate the CoreAudio loopback capture adapter with zero warnings.
10. As a streamer, I want hybrid TCP/UDP transmission over USB cable and local Wi-Fi, delivering uncompressed PCM audio frames with sub-5ms latency.

---

## Implementation Decisions

### CoreAudio Virtual HAL Adapter (`crates/server`)
- **Port Implementation**: Refactor `MacAudioCapture` implementing `AudioCapturePort` using `cpal` over macOS CoreAudio HAL.
- **Auto-Discovery Hierarchy**:
  1. Explicit CLI argument (`--device <name>`) or `AUDIO_DEVICE` environment variable.
  2. Exact match on **`BlackHole 16ch` (Default)**.
  3. Match on `BlackHole 2ch` or `BlackHole 64ch`.
  4. Any device containing `blackhole` or `loopback` (case-insensitive).
  5. Fallback to default input device with prominent console warning and Homebrew installation instructions (`brew install blackhole-16ch`).
- **Dependency Purge**:
  - Remove `screencapturekit = "8.0.1"` from `crates/server/Cargo.toml`.
  - Delete `crates/server/build.rs` (Swift runtime and toolchain rpath linking script).

### Multi-Channel Processing & Sample Rate Management
- **Sample Rate Extraction**: Query native device configuration at runtime via `device.default_input_config()` and pass the active `sample_rate` (e.g. 44100, 48000, 96000, 192000) directly into `AudioBuffer` and `AudioPacketHeader`.
- **Channel Downmixing Algorithm (ITU-R BS.775)**:
  - 2 channels: Bit-exact direct copy to Left and Right channels.
  - 6 channels (5.1 Surround): $L = L + 0.7071 \cdot C + 0.7071 \cdot Ls$, $R = R + 0.7071 \cdot C + 0.7071 \cdot Rs$.
  - 16 channels: Extract main stereo bus (channels 1-2) or apply surround downmix if surround channels are active, normalized to prevent digital clipping.

### Android Playback Synchronization (`crates/client`)
- **Dynamic AAudio Configuration**: Configure `AudioStreamBuilder` with `set_sample_rate(oboe::Unspecified)` or matching incoming packet sample rate, allowing Android audio HAL to select optimal DAC clock rates.

---

## Testing Decisions

- **Adapter Unit Tests**: Validate device enumeration and selection logic prioritizing `BlackHole 16ch` in `crates/server/src/adapters/capture_macos.rs`.
- **End-to-End Loopback Test**: Run `tests/loopback_test.rs` ensuring bit-exact frame transmission between server and client over local loopback.
- **Channel Downmix Unit Tests**: Verify that stereo and multi-channel input blocks produce mathematically accurate stereo buffers without clipping.

---

## Out of Scope

- Automated root/sudo driver installer inside the server binary (users install via standard Homebrew).
- Modifying macOS System Settings programmatically (users select BlackHole 16ch in standard sound output settings).

---

## Further Notes

- BlackHole runs in user-space CoreAudio HAL without requiring macOS kernel extensions (`kexts`) or disabling System Integrity Protection (SIP).
- Compatible with all Apple Silicon (M1/M2/M3/M4) and Intel Macs running macOS 10.10 through macOS 15+.
