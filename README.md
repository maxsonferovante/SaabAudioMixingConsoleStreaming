# Saab Audio Mixing Console Streaming

Low-latency digital audio mixing console and streaming system in Rust. Inspired by the precision cockpit ergonomics and aeronautical engineering of the iconic **Saab 900 Turbo**, this project transmits uncompressed high-resolution PCM audio from macOS to Android 12+ devices over UDP and USB, routing output to external sound systems via 3.5mm P2/auxiliary interfaces, with bidirectional WebSocket control and a dedicated studio touch console.

---

## Overview

SaabAudioMixingConsoleStreaming is designed for real-time, zero-lag audio routing between a macOS workstation and an Android receiver device. It captures master system audio digitally through macOS CoreAudio HAL and the **BlackHole 16ch** virtual audio loopback driver with zero additional latency, applies avionics-grade digital signal processing (DSP) in domain space, and streams bit-exact audio over local networks and direct USB connections.

### Architecture Highlights

- **Pure PCM UDP/USB Streaming**: Bit-exact high-resolution stereo float32 audio transmission (44.1kHz to 192kHz) with sub-5ms network latency.
- **Hexagonal Architecture (Ports and Adapters)**: Strict decoupling of domain logic, communication contracts, and platform drivers.
- **Zero-Latency CoreAudio Loopback**: Integration with `BlackHole 16ch` (and 2ch/64ch variants) for clean digital loopback capture without screen recording indicators, GPU compositor overhead, or audio echoing on Mac speakers.
- **Broadcast ITU-R BS.775 Downmixing**: Automatic weighted downmixing from 16-channel and surround (5.1/7.1) sources to clean stereo for physical P2 line outputs.
- **Android Low-Latency Output**: Dedicated Google Oboe (AAudio) engine configured for exclusive low-latency operation targeting 3.5mm P2 and USB-C DAC outputs on Android 12+ (API 31+).
- **DSP Engine**: Broadcast-standard logarithmic faders (-inf to +6dB) with frame-accurate 5ms linear gain ramps to eliminate transient pop and click artifacts during volume transitions.
- **Continuous Telemetry & Metering**: 60fps RMS and True Peak metering with exponential peak hold decay, clipping detection, and round-trip time (RTT) calculation via WebSocket ping/pong messages.
- **Touch Interface**: Studio console interface built with the `iced` GUI toolkit, suitable for desktop and mobile form factors.

---

## Core Libraries and Dependencies

The following core libraries power the streaming, audio DSP, networking, and user interface layers of the application:

| Library | Role in Architecture | Repository |
| :--- | :--- | :--- |
| **`cpal`** | Cross-platform audio I/O in pure Rust. Interfaces directly with macOS CoreAudio HAL to capture zero-latency digital loopback streams from BlackHole. | [github.com/RustAudio/cpal](https://github.com/RustAudio/cpal) |
| **`oboe` / `oboe-sys`** | Rust bindings for Google's high-performance C++ Oboe audio library on Android. Provides direct AAudio exclusive access for lowest possible hardware latency (<5ms) and hardware routing to the 3.5mm P2 audio jack. | [github.com/google/oboe](https://github.com/google/oboe) |
| **`ringbuf`** | Lock-free Single-Producer Single-Consumer (SPSC) circular buffer. Bridges real-time, non-blocking audio capture/playback callbacks with asynchronous network tasks with zero allocation and zero lock contention. | [github.com/agerasev/ringbuf](https://github.com/agerasev/ringbuf) |
| **`tokio`** | Event-driven, asynchronous I/O runtime. Manages non-blocking UDP audio packet reception, high-throughput network streaming, and concurrent WebSocket servers and clients. | [github.com/tokio-rs/tokio](https://github.com/tokio-rs/tokio) |
| **`tokio-tungstenite`** | Lightweight, high-performance asynchronous WebSocket implementation for Tokio. Handles real-time bidirectional synchronization of fader levels, mute/dim controls, and telemetry broadcasts. | [github.com/snapview/tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) |
| **`iced`** | Cross-platform GUI framework inspired by Elm. Powers the tactile studio mixing console interface with dark studio aesthetics, 20-segment responsive VU meters, and illuminated control switches. | [github.com/iced-rs/iced](https://github.com/iced-rs/iced) |
| **`serde` / `serde_json`** | Zero-copy serialization and deserialization framework. Powers serialization of WebSocket control commands, VU meter telemetry payloads, and latency diagnostics. | [github.com/serde-rs/serde](https://github.com/serde-rs/serde) |
| **`byteorder`** | Low-level binary decoding and encoding utilities. Used for bit-level packing and unpacking of the 28-byte UDP `AudioPacketHeader` and IEEE 754 float32 PCM samples. | [github.com/BurntSushi/byteorder](https://github.com/BurntSushi/byteorder) |
| **`thiserror`** | Ergonomic macro for creating typed error definitions. Manages domain and adapter error hierarchies (`CoreError`, `ProtocolError`) without runtime overhead. | [github.com/dtolnay/thiserror](https://github.com/dtolnay/thiserror) |

---

## BlackHole Variants Guide

The acronym **"ch"** stands for independent **Audio Channels**. All variants operate in macOS CoreAudio HAL with **zero additional driver latency**:

| Version | Channel Count | Production Use Case | Recommended for Project? |
| :--- | :--- | :--- | :--- |
| **`BlackHole 16ch`** | **16 Independent Channels** | Music production in DAWs (Logic Pro, Ableton, Reaper), advanced OBS multitrack routing, distinct sub-mixes (Game, Discord, Music), and 5.1/7.1 surround audio feeds. | **PROJECT STANDARD (Recommended)** |
| **`BlackHole 2ch`** | **2 Channels (Stereo: Left & Right)** | Standard stereo playback, Spotify, YouTube, common gaming, voice calls, and basic streaming. | Full Compatibility (Auto-Discovery) |
| **`BlackHole 64ch` / `256ch`** | **64 / 256 Independent Channels** | Large-scale recording studios, multi-instrument orchestral tracking, and complex industrial audio arrays. | Full Compatibility (Auto-Discovery) |

---

## System Architecture

```
+-------------------------------------------------------------+
|                         macOS Server                        |
|  - BlackHole 16ch CoreAudio HAL Loopback Stream             |
|  - ITU-R BS.775 Broadcast Downmixer (16ch/5.1 -> Stereo)    |
|  - MixerService DSP (Volume Curves, Gain Ramp, Mute State)  |
|  - VU Meter Processor & Telemetry Broadcaster               |
+------------------------------+------------------------------+
                               |
            +------------------+------------------+
            |                                     |
   [UDP/USB Audio Stream]                 [WebSocket Control]
   Raw PCM Float32 (44.1k - 192kHz)       Bidirectional Channel
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
SaabAudioMixingConsoleStreaming/
├── crates/
│   ├── protocol/        # Binary UDP packet header and WebSocket JSON DTOs
│   ├── core/            # Domain entities, DSP value types, and Ports
│   ├── server/          # macOS backend: CoreAudio BlackHole capture, UDP streamer, WebSocket server
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
- **macOS Audio Driver**: Install BlackHole 16ch via Homebrew:
  ```bash
  brew install blackhole-16ch
  ```
- **Android Target** (for Android compilation):
  - `cargo install cargo-ndk`
  - `rustup target add aarch64-linux-android`
  - Android NDK (r25 or later) with environment variable `ANDROID_NDK_HOME` configured

---

### Running the Server (macOS)

1. Set your macOS audio output to **BlackHole 16ch** in **System Settings -> Sound -> Output**.
2. Execute the server binary on macOS, pointing to the Android device IP:

```bash
cargo run --bin server -- <ANDROID_DEVICE_IP>:48480
```

- **Audio Capture**: Automatically auto-discovers and binds to **BlackHole 16ch** (or available variant).
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

The script configures ADB port forwarding (`adb forward tcp:48480 tcp:48480` and `adb forward tcp:9001 tcp:9001`), compiles the client crate using `cargo-ndk` with release optimizations, pushes the binary and `libc++_shared.so` to `/data/local/tmp/`, and initiates the low-latency Oboe playback engine routed to the 3.5mm P2 jack.

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
| `20..24` | `u32` | `sample_rate` | Sampling rate in Hz (`44100`..=`192000`) |
| `24..26` | `u16` | `channels` | Channel count (`2` for Stereo) |
| `26..27` | `u8` | `format` | Sample format identifier (`0` = Float32 LE) |
| `27..28` | `u8` | `reserved` | Reserved for byte alignment |
| `28..N` | `[f32]` | `payload` | Interleaved stereo audio samples |

---

## License

This project is licensed under the [MIT License](LICENSE).
