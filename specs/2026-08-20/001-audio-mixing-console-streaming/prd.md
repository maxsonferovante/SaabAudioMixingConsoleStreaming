# PRD 001: Audio Mixing Console & Low-Latency Streaming

## Problem Statement

Live streamers, audio creators, and power users often need a dedicated tactile audio control surface (physical faders, mute switches, real-time VU monitoring) to manage computer system audio during live broadcasts. Commercial hardware mixing consoles and stream decks can be expensive and inflexible. 

The user possesses a **MacBook** (primary workstation) and an **Android smartphone (Android 12+)** with a physical 3.5mm P2 audio jack (or P2 adapter) connected to an external speaker. However, there is no unified, high-performance, low-latency software ecosystem that enables the Android device to act simultaneously as a physical touch control surface and an ultra-low-latency wireless audio receiver that pipes system audio directly into the external speaker.

## Solution

A high-performance, 100% Rust-based distributed audio mixing system designed with strict **Clean Architecture (Hexagonal / Ports & Adapters)** and **Domain-Driven Design (DDD)** principles:
1. **Mac Server Backend (macOS)**: Implements driving adapters for macOS ScreenCaptureKit (capturing aggregate system audio with zero kernel drivers) and WebSocket control, a pure domain DSP core for volume/gain/metering, and driven adapters for UDP audio streaming and WebSocket telemetry broadcast.
2. **Mobile Client Console (Android 12+)**: A native Rust application utilizing an `iced` driving adapter for a studio-grade dark-mode touch console (vertical faders, stereo VU meters, mute/dim toggles, connection telemetry) and a Google `oboe` (AAudio) driven adapter for low-latency playback to the 3.5mm P2 audio jack.
3. **Shared Protocol Contract**: Boundary DTOs and binary UDP audio packet specifications.

---

## User Stories

1. As a streamer, I want to control the master volume of my Mac directly from my Android touchscreen fader, so that I don't have to Alt-Tab or open audio control panels during a broadcast.
2. As a streamer, I want to hear my Mac's audio output on the speaker plugged into my Android device's P2 jack with imperceptible latency (<10ms), so that my stream audio and monitoring remain perfectly in sync with video.
3. As a creator, I want real-time dual stereo VU meters (Peak and RMS) on the phone interface with saturation/clipping alerts, so that I can prevent audio distortion and maintain broadcast levels.
4. As a streamer, I want an instant, dedicated Mute button with visual illumination feedback, so that I can silence audio immediately during coughing fits or private conversations.
5. As a streamer, I want a Dim (-20dB) toggle button, so that I can quickly lower monitoring volume while talking without losing my calibrated fader position.
6. As an audio engineer, I want logarithmic audio fader curves with dB readouts (-inf to +6dB), so that volume adjustments feel natural and precise to human hearing.
7. As a mobile user, I want smooth anti-pop/click gain ramps on mute/unmute events, so that sudden sound cutoffs do not produce annoying audio artifacts in the speaker.
8. As a user on a local Wi-Fi network, I want to see real-time network telemetry (estimated latency in ms, packet loss, jitter buffer health, connection status) on the phone, so that I can diagnose network congestion.
9. As a developer, I want to capture system audio on macOS without needing to install third-party kernel extensions or virtual audio drivers like BlackHole, so that setup is zero-friction and secure.
10. As a streamer, I want automatic reconnection between the mobile console and the Mac server if Wi-Fi temporarily drops, so that the console recovers without restarting the apps.
11. As a mobile user, I want the Android app to prevent the screen from sleeping while active, so that my faders and meters remain accessible throughout the entire stream.
12. As a developer, I want the client app to run in a desktop simulator mode on macOS during development, so that I can rapidly iterate on UI and DSP without building an APK for every minor tweak.
13. As a developer, I want automated build scripts targeting Android NDK (`cargo-ndk`), so that deploying release APKs to any connected Android 12+ device is a single-command operation.

---

## Implementation Decisions

### Hexagonal Architecture & Clean Separation
- **Domain Layer (`crates/core::domain`)**: Pure, immutable Value Objects (`DecibelVolume`, `LinearGain`, `VuMeterReading`, `AudioBuffer`) with business invariants (clamping, log/linear conversions, anti-pop ramps). Zero external dependencies or framework annotations.
- **Application Layer (`crates/core::ports`)**:
  - **Primary Ports**: `ProcessAudioUseCase`, `AdjustVolumeUseCase`, `ToggleMuteUseCase`, `StreamTelemetryUseCase`.
  - **Secondary / Driven Ports**: `AudioCapturePort`, `AudioPlaybackPort`, `AudioStreamerPort`, `TelemetryBroadcasterPort`.
- **Infrastructure / Adapters (`crates/server`, `crates/client`)**:
  - *Driving Adapters*: macOS ScreenCaptureKit event tap, WebSocket command listener, Iced touch UI.
  - *Driven Adapters*: Google Oboe/AAudio output engine, UDP packetizer, Tokio WebSocket broadcaster.
- **Dependency Rule**: Adapters depend inward on Application Ports and Domain. The core domain has no knowledge of Iced, Tokio, Oboe, or ScreenCaptureKit.

### Concurrency & Real-Time Audio (Rust 1.75+)
- **Lock-Free RingBuffer Isolation**: Real-time audio threads communicate with Tokio async tasks through lock-free SPSC ring buffers (`ringbuf`), preventing priority inversion and lock contention.
- **Type Safety**: Newtype wrappers (`DecibelVolume`, `LinearGain`) prevent unit mismatches at compile time.

---

## Testing Decisions

- **Domain Isolation Tests**: Unit tests executing 100% in memory with no I/O, testing DSP mathematics, dB conversions, and anti-pop transitions.
- **Mock-Driven Use Case Tests**: Verification of primary ports against mock implementations of secondary ports (`MockAudioStreamerPort`, `MockTelemetryBroadcasterPort`).
- **Protocol Serialization Tests**: Round-trip verification for binary UDP frames and WebSocket JSON messages.
- **Desktop Loopback Integration Test**: End-to-end simulation on macOS testing server and client over local loopback (`127.0.0.1`).

---

## Out of Scope

- **Amazon Echo / Alexa Integration**: Explicitly deferred to future phases (YAGNI).
- **Multi-Track / Per-App Physical Separation**: Individual per-application volume sliders (captures global system output mix for MVP).
- **Cloud / WAN Streaming**: Operation is strictly localized to LAN / Wi-Fi / USB network interfaces.
- **Microphone Reverse Routing**: Routing the phone's microphone back to the Mac is not in scope for this milestone.

---

## Further Notes

- **Device Compatibility**: Dispositivos Android rodando Android 12 ou superior (API level 31+), com suporte nativo ao backend AAudio / Oboe de baixa latência e conector 3.5mm P2 (ou adaptador P2/USB-C).
- **Connection Optimization**: While Wi-Fi UDP provides <5ms latency, connecting the USB-C cable to the Mac allows USB Tethering (RNDIS) for absolute 0ms jitter and simultaneous device charging.
