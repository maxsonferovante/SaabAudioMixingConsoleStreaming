# 02 — CoreAudio HAL Dynamic Auto-Follower & Hot-Migration

**What to build:** A background supervisor thread in `MacAudioCapture` that queries `host.default_output_device()` on a 250ms interval, dynamically detects when the user changes their macOS sound output device in System Settings (e.g., between `BlackHole 2ch`, `BlackHole 16ch`, or `BlackHole 64ch`), and safely rebinds the CPAL input stream in under 100ms without restarting the server or interrupting active network streams.

**Blocked by:** 01 — Smart 16-Channel Downmixer & Anti-Clipping Soft Limiter

**Status:** done

- [x] Supervisor thread continuously monitors active macOS sound output without blocking real-time audio threads.
- [x] Switching macOS sound output triggers automatic CPAL stream re-creation with target device sample rate and channel count.
- [x] Safe shutdown on `stop_capture()` with clean resource cleanup and zero thread leaks.
