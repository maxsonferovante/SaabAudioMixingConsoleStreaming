## What to build

A continuous audio metering subsystem (VU Meter with RMS, True Peak, and clipping/saturation detection) and network telemetry (RTT in milliseconds, packet rates, and jitter buffer health), broadcast at 60fps via WebSocket for precise visual monitoring on the client console.

## Acceptance criteria

- [x] Domain value object `VuMeterReading` calculating RMS (root-mean-square energy) and absolute True Peak per stereo block (L and R channels).
- [x] Clipping detection and signaling when peak levels exceed 0.995 (-0.04 dBFS) with smooth exponential peak hold decay (`VuMeterState`).
- [x] Periodic telemetry messages at 60fps broadcast via WebSocket from server to client.
- [x] Network health monitoring on the client (round-trip time RTT ping/pong, reception buffer fill percentage).
- [x] Unit tests for RMS and Peak calculations using calibrated sine test signals.

## Blocked by

- [03 — Domain DSP Engine and Fader/Mute Synchronization via WebSocket](03-dsp-fader-websocket-sync.md)
