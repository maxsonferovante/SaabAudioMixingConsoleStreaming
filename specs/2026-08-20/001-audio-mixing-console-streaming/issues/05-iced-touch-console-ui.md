## What to build

A modern, professional audio mixing console user interface built with the `iced` GUI library featuring a Studio Dark theme, responsive vertical tactile fader, dual dynamic stereo VU meters with color gradients (Green -> Yellow -> Red), illuminated tactile Mute and Dim buttons, and a top telemetry bar with real-time connection status and latency.

## Acceptance criteria

- [x] Custom vertical Fader visual component with tactile drag support, displaying decibel scale (-inf, -60dB to +6dB).
- [x] Dual stereo VU Meter component with 60fps rendering, color-gradient segment bars, and clipping peak indicator.
- [x] Tactile buttons with active illuminated feedback for Mute (red), Dim (amber), and 0 dB unity gain reset.
- [x] Top telemetry status panel showing connection state (Connected / Disconnected), host address, and estimated RTT latency in milliseconds.
- [x] Support for desktop execution (macOS) and responsive layout preparation for vertical smartphone screens.

## Blocked by

- [04 — 60fps Stereo VU Meters and Network Telemetry (RTT, Jitter)](04-vu-meters-telemetry.md)
