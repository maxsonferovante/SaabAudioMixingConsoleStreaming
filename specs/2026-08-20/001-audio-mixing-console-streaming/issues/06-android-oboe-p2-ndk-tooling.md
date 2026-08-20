## What to build

An ultra-low latency audio output adapter for Android using Google `oboe` (AAudio) in exclusive high-performance mode, consuming packets from the UDP jitter reception buffer and routing analog audio to the 3.5mm P2 connector (or P2/USB-C adapter), along with build automation scripts for NDK (`cargo-ndk`) and ADB deployment on Android 12+ devices.

## Acceptance criteria

- [x] Audio playback adapter `OboeAudioPlayback` using the `oboe` crate for low-latency audio output in exclusive mode (`PerformanceMode::LowLatency`).
- [x] Audio routing configured for headphone / external line-out output (3.5mm P2 connector).
- [x] Lock-free sample consumption from the circular jitter ring buffer in the real-time AAudio callback.
- [x] Automation script `scripts/build_android.sh` configured to build the `client` crate for `aarch64-linux-android` architecture using `cargo-ndk` and install via `adb`.
- [x] Test and validation of continuous drop-free streaming between macOS and Android smartphones over the local network.

## Blocked by

- [05 — Tactile Iced GUI (Studio Dark Console)](05-iced-touch-console-ui.md)
