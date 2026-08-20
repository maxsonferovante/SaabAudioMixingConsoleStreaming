# SPEC 001: Audio Mixing Console & Low-Latency Streaming System

## Problem Statement

Content creators and live streamers operating macOS workstations require dedicated tactile control over their master and application audio levels without switching context away from their broadcast or full-screen workflow. Existing hardware mixers and stream decks introduce substantial hardware cost, lack flexibility, and require physical desk footprint.

At the same time, the user owns an **Android device (running Android 12 or newer)** equipped with a physical 3.5mm P2 audio jack (or adapter) connected to an external speaker. However, there is no integrated, high-performance solution that transforms this Android device into a synchronized wireless tactile control surface while concurrently receiving and streaming computer system audio to the external speaker with sub-10ms latency.

## Solution

A modular, distributed audio mixing and low-latency streaming platform built 100% in modern Rust (1.75+) following strict **Clean Architecture (Hexagonal / Ports & Adapters)** and **Domain-Driven Design (DDD)** principles:

1. **Mac Server Hub (macOS)**: 
   - Captures real-time digital system audio with zero third-party drivers using native macOS ScreenCaptureKit.
   - Executes DSP pipeline (logarithmic broadcast volume faders, click-free gain ramp transitions, dual stereo RMS and Peak VU meter computation with clipping detection).
   - Streams audio frames via low-latency UDP datagrams (<5ms network latency) to the mobile device.
   - Hosts a bidirectional WebSocket control & telemetry server for deterministic state synchronization at 60fps.
2. **Mobile Client Console (Android 12+)**:
   - Renders a touch-optimized studio mixing console using the `iced` GUI framework.
   - Receives UDP audio packets into a lock-free jitter ring buffer.
   - Outputs low-latency audio via Google `oboe` (AAudio engine) through the physical 3.5mm P2 audio jack.
   - Transmits real-time fader positions and mute states back to the Mac server.
3. **Shared Protocol Contract**:
   - Provides binary UDP packet definitions and structured WebSocket telemetry/command schemas.

---

## User Stories

1. As a streamer, I want to slide a tactile master fader on my Android device to adjust macOS system volume instantly, so that I can balance sound levels without interrupting my live broadcast.
2. As a streamer, I want to hear my Mac's audio output on the speaker connected to my Android phone's P2 jack with imperceptible latency (<10ms), so that game sound and voice monitoring remain perfectly aligned with video.
3. As an audio creator, I want real-time dual stereo VU meters (Peak and RMS) on the mobile display with visual clipping indicators, so that I can prevent digital distortion and monitor broadcast loudness.
4. As a streamer, I want a dedicated, illuminated Mute button on the phone console, so that I can silence master audio immediately with visual confirmation.
5. As a streamer, I want a Dim (-20dB) toggle switch, so that I can temporarily reduce output volume during conversations without losing my calibrated fader level.
6. As an audio engineer, I want logarithmic fader scaling with precise decibel readouts (-inf to +6dB), so that slider travel feels intuitive and proportional to human loudness perception.
7. As a listener, I want smooth anti-pop/click gain ramps applied during mute and unmute transitions, so that sudden audio cutoffs do not produce abrasive clicks in the physical speaker.
8. As a user operating over local Wi-Fi, I want visible network telemetry (estimated round-trip latency, jitter buffer occupancy, packet drop counter, server connection status), so that I can monitor connection stability.
9. As a macOS user, I want system audio captured natively without installing third-party virtual audio cables or kernel extensions, so that the application is secure and easy to set up.
10. As a mobile user, I want the client app to automatically reconnect if Wi-Fi temporarily drops, so that audio and control resume seamlessly without manual intervention.
11. As a streamer, I want the mobile application to prevent the screen from locking while active, so that my faders and meters remain immediately accessible throughout long sessions.
12. As a developer, I want the client console to execute directly on macOS in desktop simulator mode, so that I can rapidly test and iterate on UI and DSP logic without deploying an APK for each change.
13. As a developer, I want automated compilation scripts targeting Android NDK (`cargo-ndk`), so that generating and deploying release APKs to connected Android devices is fully automated.

---

## Implementation Decisions

### Architectural Seams & Hexagonal Boundaries

The codebase enforces a strict inside-out dependency rule: Domain Core has zero external dependencies, Application Ports define abstract contracts, and Adapters implement concrete I/O.

```
[ Driving Adapters ]                       [ Driven Adapters ]
  - ScreenCaptureKit (macOS)                 - Google Oboe / AAudio (Android)
  - WebSocket Command Listener               - UDP Audio Streamer
  - Iced Touch GUI                           - WebSocket Telemetry Broadcaster
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

### Native macOS System Audio Capture (ScreenCaptureKit)

- **Option 1: Native Driverless Capture via Apple ScreenCaptureKit (`SCStream`)**:
  - Interfaces directly with Apple's `ScreenCaptureKit` (`SCStream` with `capturesAudio = true`) on macOS 13+.
  - Captures bit-exact system-wide digital audio (Spotify, browsers, games, media) in 48kHz stereo float32 PCM without requiring third-party virtual audio drivers (such as BlackHole or Perssua).
  - Requires *Screen & System Audio Recording* permission in macOS *System Settings -> Privacy & Security*.
  - Provides seamless, transparent fallback to `cpal` hardware/virtual audio input if recording permissions are pending or if an explicit audio device is passed via CLI.

### Domain Model & Invariants
- **Volume Representation**: Encoded as immutable `DecibelVolume` (-80.0 dB to +6.0 dB, with -80.0 dB representing $-\infty$) mapped to `LinearGain` via $g = 10^{\frac{\text{dB}}{20}}$.
- **Anti-Pop Smoothing**: Gain changes transition across a 5ms linear interpolation window (240 samples at 48kHz) to eliminate DC offset clicks.
- **VU Meter Calculation**: Computes true Peak sample absolute maximum and RMS (Root Mean Square) energy over 1024-sample windows (21.3ms), flagging clipping whenever sample peak exceeds 0.995 (-0.04 dBFS).

### Prototype Type Shapes & State Contracts

```rust
// Core Domain Value Objects
pub struct DecibelVolume(f32); // Invariant: -80.0..=6.0
pub struct LinearGain(f32);    // Invariant: 0.0..=2.0

pub struct VuMeterReading {
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
    pub is_clipping: bool,
}

// WebSocket Control & Telemetry Protocol
pub enum ControlCommand {
    SetMasterVolume { db: f32 },
    ToggleMute { muted: bool },
    ToggleDim { dimmed: bool },
    Ping { timestamp: u64 },
}

pub enum TelemetryPacket {
    VuMeter(VuMeterReading),
    ConnectionStats { rtt_ms: f32, buffer_fill_pct: f32 },
    Pong { timestamp: u64 },
}

// UDP Audio Frame Header (Binary Protocol)
#[repr(C)]
pub struct AudioPacketHeader {
    pub sequence_number: u64,
    pub timestamp_us: u64,
    pub sample_count: u16,
    pub channels: u8, // 2 = Stereo
    pub sample_rate: u32, // 48000
}
```

### Concurrency & Real-Time Isolation
- **Lock-Free Audio Threads**: Real-time CoreAudio / Oboe callbacks interact with network workers exclusively via single-producer single-consumer (SPSC) lock-free ring buffers (`ringbuf`), preventing mutex contention or priority inversion on real-time audio threads.
- **Async I/O Tasks**: Tokio manages WebSocket connections, network discovery, and telemetry broadcasts asynchronously.

---

## Testing Decisions

### Test Seams
1. **Primary Application Seam (`MixerService`)**:
   - Test all DSP calculations, anti-pop gain ramps, and volume curves by invoking `ProcessAudioUseCase` in memory with synthetic test buffers.
   - Assert on secondary mock ports (`MockAudioStreamerPort`, `MockTelemetryBroadcasterPort`) without spinning up real audio hardware or network sockets.
2. **Protocol Serialization Seam**:
   - Verify zero data loss and exact byte layout for UDP binary packet encoding and WebSocket JSON payloads.
3. **End-to-End Loopback Seam**:
   - Test complete integration on macOS by running the server and client in desktop mode over `127.0.0.1`.

### Good Test Principles
- Tests verify **external observable behavior** (e.g., gain attenuation, RMS correctness, packet arrival) rather than internal private variables.
- Domain tests run completely in memory in milliseconds with zero flaky I/O.

---

## Out of Scope

- **Amazon Echo / Alexa Integration**: Deferred to future phases (YAGNI).
- **Per-Application Audio Routing**: Captures global system output mix for MVP.
- **Wide Area Network (WAN) Streaming**: Traffic is restricted to local LAN / Wi-Fi / USB interfaces.
- **Mobile Microphone Passthrough**: Reverse audio routing from Android to Mac is out of scope for this milestone.

---

## Further Notes

- **Target Platforms**: macOS 12.3+ (Apple Silicon & Intel) for backend; Android 12+ (API 31+) for mobile console with fallback desktop support.
- **Network Optimization**: While Wi-Fi UDP delivers <5ms latency, USB-C tethering (RNDIS) provides 0ms jitter and simultaneous device charging.
