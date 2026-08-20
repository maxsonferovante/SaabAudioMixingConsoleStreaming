# SPEC 002: BlackHole 16ch CoreAudio Virtual Driver Architecture & Specification

## Problem Statement

The initial macOS audio capture mechanism leveraged Apple's `ScreenCaptureKit` (`SCStream`). Although functional without virtual drivers, ScreenCaptureKit imposes significant runtime drawbacks for studio-grade audio streaming, mixing, and multitrack workflows:

1. **System Indicator & Permission Intrusion**: ScreenCaptureKit triggers the system-wide macOS screen recording indicator in the menu bar and requires persistent display capture permissions, creating privacy confusion and visual clutter.
2. **Video Compositor Overhead**: ScreenCaptureKit operates inside the window server graphics pipeline, unnecessarily capturing video display frames alongside audio, consuming GPU and CPU cycles.
3. **Sound Duplication & Lack of Output Isolation**: ScreenCaptureKit taps the audio signal at the final screen compositor stage, causing audio to play simultaneously out of the Mac physical speakers and the remote Android receiver.
4. **Channel & Multitrack Constraints**: ScreenCaptureKit restricts capture to 2-channel stereo, preventing advanced studio workflows such as DAW multitrack routing (Logic Pro, Ableton, Reaper), multitrack broadcast streaming (OBS Studio), and 5.1/7.1 surround audio feeds.

## Solution

Migrate the macOS audio capture pipeline to **BlackHole 16ch** as the primary default CoreAudio virtual audio driver:

1. **BlackHole 16ch Default Standard**: Adopt `BlackHole 16ch` as the primary standard driver for the system, enabling simultaneous routing of master system audio, discrete DAW channels, game audio, communications (Discord/Zoom), and surround streams.
2. **Pure CoreAudio HAL Virtual Driver Integration**: Directly captures the virtual loopback stream from BlackHole via `cpal` and macOS CoreAudio HAL with zero additional driver latency.
3. **Swift Runtime & ScreenCaptureKit Purge**: Removes `screencapturekit` crate dependencies and deletes `build.rs` Swift linking scripts, reducing build times and binary overhead.
4. **Intelligent Device Auto-Discovery**: Automatically enumerates and binds to `BlackHole 16ch` by default, with seamless compatibility for `BlackHole 2ch` and `BlackHole 64ch`, falling back gracefully with clear Homebrew installation instructions (`brew install blackhole-16ch`).
5. **High-Resolution Dynamic Sample Rates**: Supports native sample rates (44.1kHz, 48kHz, 88.2kHz, 96kHz, 176.4kHz, 192kHz) and encodes the active rate in binary packet headers.
6. **Standard ITU-R BS.775 Broadcast Downmix**: Provides weighted surround downmixing for multi-channel feeds (5.1/7.1/16ch) and bit-exact stereo passthrough for 2-channel inputs.
7. **Exclusive Sound Routing**: Directing macOS system output to BlackHole isolates system audio entirely, transmitting it exclusively to the Android P2 speaker output.

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
3. As a user, I want the server to auto-discover `BlackHole 16ch` by default on launch, so that no manual configuration is required.
4. As a user who has not yet installed BlackHole, I want the server to output copy-pasteable Homebrew commands (`brew install blackhole-16ch`), so that I can set up the driver quickly.
5. As an audio professional, I want to stream high-resolution 96kHz or 192kHz audio from my 16-channel DAW session without downsampling, so that studio-grade audio fidelity is preserved.
6. As a user playing surround 5.1/7.1 content on `BlackHole 16ch`, I want automatic ITU-R BS.775 broadcast downmixing to stereo, so that center dialogue and rear surround effects remain balanced without phase cancellation.
7. As an Android user, I want the Google Oboe AAudio playback engine to automatically adapt to the incoming sample rate, so that audio playback is pitch-perfect without drift.
8. As a developer, I want the `crates/server` crate to compile in pure safe Rust without Swift runtime linking scripts, so that compilation is fast and portable.
9. As a developer, I want all workspace unit tests and integration tests to validate the CoreAudio loopback capture adapter with zero warnings.
10. As a streamer, I want hybrid TCP/UDP transmission over USB cable and local Wi-Fi, delivering uncompressed PCM audio frames with sub-5ms latency.

---

## Implementation Decisions

### Architectural Seams & Hexagonal Boundaries

The system maintains strict Ports and Adapters boundaries. The `AudioCapturePort` secondary port remains unchanged in domain core, while the driving capture adapter in `crates/server` is rewritten to interface exclusively with CoreAudio HAL.

```
[ Driving Adapters ]                       [ Driven Adapters ]
  - BlackHole CoreAudio HAL (macOS)          - Google Oboe / AAudio (Android)
  - WebSocket Command Listener               - Hybrid TCP/UDP Audio Streamer
  - Iced Studio Touch GUI                    - WebSocket Telemetry Broadcaster
           │                                          ▲
           ▼                                          │
   ┌──────────────────────────────────────────────────────┐
   │                  APPLICATION PORTS                   │
   │  Primary: ProcessAudio, AdjustVolume, ToggleMute     │
   │  Secondary: AudioCapture, AudioPlayback, AudioStream │
   │  ──────────────────────────────────────────────────  │
   │                     DOMAIN CORE                      │
   │  Value Objects: DecibelVolume, LinearGain, VuMeter   │
   │  Aggregates: MixerChannel, AudioBuffer               │
   └──────────────────────────────────────────────────────┘
```

### CoreAudio Device Auto-Discovery Protocol

The discovery sequence queries the default audio host and prioritizes devices as follows:

1. **CLI / Environment Override**: `--device <name>` or `AUDIO_DEVICE` environment variable.
2. **Exact Standard Match**: **`BlackHole 16ch` (Default)**.
3. **Alternative Variant Matches**: `BlackHole 2ch` -> `BlackHole 64ch`.
4. **Generic Loopback Matches**: Any device containing `blackhole` or `loopback` (case-insensitive).
5. **Fallback & Guidance**: If no virtual device is found, log explicit Homebrew installation instructions and fall back to the system default input device:
   ```
   [WARN] BlackHole virtual audio driver not detected.
   [INFO] To capture system audio without screen recording permissions, install BlackHole 16ch:
   [INFO]   brew install blackhole-16ch  (Project Standard - 16-channel DAWs, surround & streaming)
   [INFO]   brew install blackhole-2ch   (Alternative - 2-channel basic stereo)
   [INFO]   brew install blackhole-64ch  (Alternative - 64-channel studio routing)
   ```

### ITU-R BS.775 Broadcast Downmixing Specification

When receiving multi-channel frames from `BlackHole 16ch` or `BlackHole 64ch`, frames are downmixed to stereo:

$$L_{\text{out}} = L + 0.7071 \cdot C + 0.7071 \cdot Ls$$
$$R_{\text{out}} = R + 0.7071 \cdot C + 0.7071 \cdot Rs$$

For 2-channel streams, frames are copied directly without computational overhead.

### Dynamic Sample Rate Propagation

The active sample rate queried from the device input configuration is embedded into each binary `AudioPacketHeader`:

```rust
// Binary Audio Packet Header Shape (28 bytes)
pub struct AudioPacketHeader {
    pub magic: [u8; 4],         // b"AMCS"
    pub sequence_number: u64,   // Monotonic counter
    pub timestamp_us: u64,      // Microsecond timestamp
    pub sample_rate: u32,       // 44100..=192000 Hz
    pub channels: u16,          // 2 (Stereo)
    pub sample_count: u16,      // Frame count per block
    pub format: SampleFormat,   // Float32 LE (0)
    pub reserved: u8,
}
```

---

## Testing Decisions

### Seam Architecture
- **Primary Testing Seam**: `AudioCapturePort` interface in `crates/core` and `MacAudioCapture` adapter in `crates/server`.
- **Characteristics of Good Tests**: Tests verify observable audio processing, channel downmix equations, sample rate extraction, and end-to-end packet delivery without depending on physical hardware presence.

### Test Matrix
1. **Downmix Precision Tests**: Unit tests asserting mathematical correctness of ITU-R BS.775 downmix for 2ch, 6ch (5.1), and 16ch configurations.
2. **Device Discovery Hierarchy Tests**: Unit tests validating device matching priority (`BlackHole 16ch` first) and fallback behaviors.
3. **End-to-End Loopback Test (`tests/loopback_test.rs`)**: Full integration test verifying uncompressed frame delivery and sample rate header round-trips.

---

## Out of Scope

- Automated root/sudo driver installer inside the server binary (users install via standard Homebrew).
- Modifying macOS System Settings programmatically (users select BlackHole 16ch in standard sound output settings).

---

## Further Notes

- BlackHole runs in user-space CoreAudio HAL without requiring macOS kernel extensions (`kexts`) or disabling System Integrity Protection (SIP).
- Compatible with all Apple Silicon (M1/M2/M3/M4) and Intel Macs running macOS 10.10 through macOS 15+.
- For in-depth mathematical formulations, psychoacoustic theory, chunk size calculations, and DAC clock synchronization details, see [`audio_theory_and_dsp_architecture.md`](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/specs/2026-08-20/002-blackhole-audio-migration/audio_theory_and_dsp_architecture.md).
