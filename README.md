# AudioMixingConsole Streaming

Low-latency digital audio mixing console and streaming system in Rust. Transmits uncompressed PCM audio from macOS to Android 12+ devices over UDP, routing output to external sound systems via 3.5mm P2/auxiliary interfaces, with bidirectional WebSocket control and a dedicated studio touch console.

---

## Overview

AudioMixingConsole Streaming is designed for real-time audio routing between a macOS workstation and an Android receiver device. It eliminates the need for virtual driver workarounds by interfacing directly with native system audio frameworks, applying digital signal processing (DSP) in domain space, and streaming bit-exact audio over a local network.

### Architecture Highlights

- **Pure PCM UDP Streaming**: Bit-exact 48kHz stereo float32 audio transmission with sub-5ms network latency.
- **Hexagonal Architecture (Ports and Adapters)**: Strict decoupling of domain logic, communication contracts, and platform drivers.
- **macOS System Capture**: Integration with ScreenCaptureKit and CoreAudio for system-wide digital audio capture.
- **Android Low-Latency Output**: Dedicated Google Oboe (AAudio) engine configured for exclusive low-latency operation targeting 3.5mm P2 and USB-C DAC outputs on Android 12+ (API 31+).
- **DSP Engine**: Broadcast-standard logarithmic faders (-inf to +6dB) with frame-accurate 5ms linear gain ramps to eliminate transient pop and click artifacts during volume transitions.
- **Continuous Telemetry & Metering**: 60fps RMS and True Peak metering with exponential peak hold decay, clipping detection, and round-trip time (RTT) calculation via WebSocket ping/pong messages.
- **Touch Interface**: Studio console interface built with the `iced` GUI toolkit, suitable for desktop and mobile form factors.

---

## System Architecture

```
+-------------------------------------------------------------+
|                         macOS Server                        |
|  - ScreenCaptureKit / CoreAudio Capture Adapter             |
|  - MixerService DSP (Volume Curves, Gain Ramp, Mute State)  |
|  - VU Meter Processor & Telemetry Broadcaster               |
+------------------------------+------------------------------+
                               |
            +------------------+------------------+
            |                                     |
   [UDP Audio Stream]                     [WebSocket Control]
   Raw PCM Float32 (48kHz)                Bidirectional Channel
   28-byte Binary Header                  - Fader / Mute Commands
   Latency: <5ms                          - 60fps VU Meters / RTT
            |                                     |
            +------------------+------------------+
                               |
                               v
+-------------------------------------------------------------+
|                    Android 12+ / Desktop Client             |
|  - Lock-free SPSC Ring Buffer (Jitter Buffer)               |
|  - Google Oboe / AAudio Exclusive Output Engine             |
|  - Iced Studio Touch Console UI                             |
+------------------------------+------------------------------+
                               |
                               v
                 [ 3.5mm P2 / Line Output ]
                               |
                               v
                    External Sound Hardware
```

---

## Workspace Layout

```
AudioMixingConsoleStreaming/
├── crates/
│   ├── protocol/        # Binary UDP packet header and WebSocket JSON DTOs
│   ├── core/            # Domain entities, DSP value types, and Ports
│   ├── server/          # macOS backend: audio capture, UDP streamer, WebSocket server
│   └── client/          # Client frontend: Iced UI, UDP receiver, CPAL & Oboe playback
├── scripts/
│   └── build_android.sh # Cargo-NDK build and ADB deployment script for Android 12+
├── specs/               # Product Requirements and Architecture Specifications
└── tests/               # End-to-end loopback and synchronization test suites
```

---

## Getting Started

### Prerequisites

- **Rust**: Version 1.75 or later (`rustup default stable`)
- **macOS**: Version 12.3 or later (for backend system audio capture)
- **Android Target** (for Android compilation):
  - `cargo install cargo-ndk`
  - `rustup target add aarch64-linux-android`
  - Android NDK (r25 or later) with environment variable `ANDROID_NDK_HOME` configured

---

### Running the Server (macOS)

Execute the server binary on macOS:

```bash
cargo run --bin server
```

- **UDP Audio Socket**: Dynamically bound, streaming to client port `48480`.
- **WebSocket Control Server**: Listening on `ws://0.0.0.0:9001`.

---

### Running the Desktop Client (macOS Simulator)

Execute the client binary locally for testing and simulation:

```bash
cargo run --bin client
```

---

### Compiling and Deploying to Android

Connect an Android 12+ device with USB debugging enabled, then execute:

```bash
./scripts/build_android.sh
```

The script performs target verification, compiles the client crate using `cargo-ndk` with full release optimizations, transfers the binary to `/data/local/tmp/client`, and initiates playback.

---

## Testing and Verification

Run the test suite across the entire workspace:

```bash
# Execute unit and integration tests
cargo test --workspace

# Execute Clippy with strict checks
cargo clippy --workspace -- -D warnings

# Verify formatting
cargo fmt --check
```

---

## Protocol Specification

### UDP Audio Datagram (`protocol`)

Each UDP datagram consists of a fixed 28-byte binary header followed by raw interleaved IEEE 754 float32 PCM samples:

| Offset | Type | Field | Description |
| :---: | :---: | :--- | :--- |
| `0..4` | `[u8; 4]` | `magic` | Identifier bytes (`b"AMCS"`) |
| `4..12` | `u64` | `sequence_number` | Monotonic packet counter |
| `12..20` | `u64` | `timestamp_us` | Microsecond timestamp for jitter tracking |
| `20..24` | `u32` | `sample_rate` | Sampling rate in Hz (`48000`) |
| `24..26` | `u16` | `channels` | Channel count (`2` for Stereo) |
| `26..27` | `u8` | `format` | Sample format identifier (`0` = Float32 LE) |
| `27..28` | `u8` | `reserved` | Reserved for byte alignment |
| `28..N` | `[f32]` | `payload` | Interleaved stereo audio samples |

---

## License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
