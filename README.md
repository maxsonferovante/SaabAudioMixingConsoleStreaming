# 🎛️ AudioMixingConsole Streaming

> **Ultra-low latency (<5ms) digital audio mixing console and streaming system in Rust.**  
> Stream raw, uncompressed PCM audio from macOS to Android 12+ devices over UDP and output to external speakers via 3.5mm P2, with real-time bidirectional WebSocket control and tactile studio UI.

---

## 🌟 Key Features

- **Pure Raw PCM Streaming (UDP)**: Bit-exact, uncompressed 48kHz stereo float32 audio streaming with low jitter and sub-5ms network latency.
- **Clean Architecture (Hexagonal / Ports & Adapters)**: Strict separation of concerns with domain purity in `audio_core`, protocol definitions in `protocol`, and infrastructure adapters in `server` and `client`.
- **macOS Native Capture**: Digital system audio capture via ScreenCaptureKit / CoreAudio with automatic fallback.
- **Android Low-Latency Output (AAudio / Oboe)**: High-performance exclusive audio playback targeting the 3.5mm P2 audio jack (or USB-C to P2 DACs) on Android 12+ devices.
- **Domain DSP & Anti-Pop Smoothing**: True logarithmic broadcast fader curves (-$\infty$ to +6dB) with exact 5ms linear gain ramps to prevent clicks and pops during volume changes and Mute/Dim toggles.
- **60fps Dual Stereo VU Meters**: Real-time RMS, True Peak, clipping indicators, and exponential peak hold decay.
- **Network Telemetry**: Continuous round-trip time (RTT) calculation via WebSocket ping/pong and buffer health monitoring.
- **Tactile Studio Dark UI**: Modern, responsive touch console built with `iced` (supports desktop and smartphone layouts).

---

## 📐 System Architecture

```
                                  +---------------------------------------+
                                  |         macOS Backend Server          |
                                  |  - ScreenCaptureKit / CoreAudio       |
                                  |  - MixerService DSP (Faders / Mute)   |
                                  |  - VU Meter Calculator & Telemetry    |
                                  +-------------------+-------------------+
                                                      |
                             +------------------------+------------------------+
                             |                                                 |
                  [UDP Audio Stream]                                 [WebSocket JSON]
                  Raw PCM Float32 (48kHz)                            Bi-directional Sync
                  Packet Header: 28 bytes                            - SetMasterVolume, SetMute
                  Latency: <5ms                                      - VU Meters (60fps), Ping/Pong
                             |                                                 |
                             +------------------------+------------------------+
                                                      |
                                                      v
                                  +---------------------------------------+
                                  |       Android 12+ / Desktop Client     |
                                  |  - Lock-Free SPSC Ring Buffer         |
                                  |  - Google Oboe / AAudio Engine        |
                                  |  - Iced Touch Console Interface       |
                                  +-------------------+-------------------+
                                                      |
                                                      v
                                            [ 3.5mm P2 / Line-Out ]
                                                      |
                                                      v
                                           🔊 External Audio Box
```

---

## 📂 Workspace Structure

```
AudioMixingConsoleStreaming/
├── crates/
│   ├── protocol/       # Binary UDP packet header (28 bytes) and WebSocket JSON DTOs
│   ├── core/           # Pure domain entities, value types, DSP volume curves, and Hexagonal Ports
│   ├── server/         # macOS backend: audio capture, UDP streamer, WebSocket server
│   └── client/         # Client frontend: Iced Dark Studio UI, UDP receiver, CPAL & Oboe AAudio
├── scripts/
│   └── build_android.sh # Automated build (cargo-ndk) and ADB deploy script for Android 12+
├── specs/              # PRD, Architecture Specs, and Technical Decisions
└── tests/              # End-to-End loopback and WebSocket integration test suites
```

---

## 🚀 Getting Started

### Prerequisites

- **Rust**: 1.75+ (`rustup default stable`)
- **macOS**: 12.3+ (for backend system audio capture)
- **Android**: Android 12+ (API 31+) device with a 3.5mm P2 jack (or USB-C audio adapter)
- **Android Tooling (for Android builds)**:
  - `cargo install cargo-ndk`
  - `rustup target add aarch64-linux-android`
  - Android NDK (r25+) installed and exported (`ANDROID_NDK_HOME`)

---

### 1. Running the Server (macOS)

Start the macOS backend server:

```bash
cargo run --bin server
```

- **UDP Audio Socket**: Bound to local port (streams to client port `48480`).
- **WebSocket Control Server**: Listening on `ws://0.0.0.0:9001`.

---

### 2. Running the Desktop Client (macOS / Simulator)

Start the desktop studio console with local audio playback and interactive UI:

```bash
cargo run --bin client
```

---

### 3. Building and Deploying to Android

Connect your Android 12+ device with USB debugging enabled, then run:

```bash
./scripts/build_android.sh
```

The script will:
1. Validate `cargo-ndk` and the `aarch64-linux-android` target.
2. Compile the client crate in release mode with LTO optimizations.
3. Push the binary to the device via ADB and execute the audio engine.

---

## 🧪 Testing & Code Quality

Run the automated test suite across all workspace crates:

```bash
# Run all unit and integration tests
cargo test --workspace

# Run Clippy linter with strict warnings
cargo clippy --workspace -- -D warnings

# Check code formatting
cargo fmt --check
```

---

## 📡 Communication Protocol

### UDP Audio Datagram (`protocol`)

Each UDP audio datagram consists of a **28-byte binary header** followed by raw IEEE 754 float32 PCM samples:

| Offset | Type | Field | Description |
| :---: | :---: | :--- | :--- |
| `0..4` | `[u8; 4]` | `magic` | Identifier bytes (`b"AMCS"`) |
| `4..12` | `u64` | `sequence_number` | Monotonic packet counter |
| `12..20` | `u64` | `timestamp_us` | Microsecond timestamp for jitter tracking |
| `20..24` | `u32` | `sample_rate` | Audio sample rate (`48000`) |
| `24..26` | `u16` | `channels` | Channel count (`2` for Stereo) |
| `26..27` | `u8` | `format` | Sample format (`0` = Float32 LE) |
| `27..28` | `u8` | `reserved` | Reserved for future alignment |
| `28..N` | `[f32]` | `payload` | Interleaved stereo audio samples |

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
