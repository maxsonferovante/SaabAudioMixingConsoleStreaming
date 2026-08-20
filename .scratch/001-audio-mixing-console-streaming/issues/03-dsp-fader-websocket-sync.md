## What to build

A pure digital signal processing (DSP) engine in `core` that applies a professional broadcast logarithmic fader curve (-inf to +6dB), a smooth 5ms anti-pop gain ramp when muting/unmuting, and a Dim button (-20dB), integrated with a WebSocket server on macOS and a WebSocket client that synchronizes state and control commands in real-time with low latency.

## Acceptance criteria

- [x] Immutable domain value types `DecibelVolume`, `LinearGain`, and `MuteState` in `core::domain`.
- [x] Implementation of linear gain interpolation over 240 samples (5ms at 48kHz) to eliminate audio clicks and pops during transitions.
- [x] WebSocket control message contracts (`ControlCommandDto`: `SetMasterVolume`, `SetMute`, `SetDim`) in `protocol`.
- [x] WebSocket server in `server` executed with Tokio processing commands and updating the DSP state at runtime.
- [x] WebSocket client in `client` sending volume updates and receiving state confirmations.
- [x] Domain unit tests verifying accurate volume attenuation and anti-pop smoothness without distortion.

## Blocked by

- [01 — Workspace Scaffolding and Pure Loopback Audio Streaming](01-scaffolding-loopback-audio.md)
