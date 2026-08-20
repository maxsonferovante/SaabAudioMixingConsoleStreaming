## What to build

A native macOS audio capture adapter in the backend (`server`) that integrates with the ScreenCaptureKit and CoreAudio APIs to capture all digital audio output emitted by the operating system and active applications (games, music, browsers, calls) without requiring third-party virtual drivers (such as BlackHole), feeding the real-time streaming pipeline.

## Acceptance criteria

- [x] Implementation of the audio capture adapter (`MacAudioCapture`) implementing the `AudioCapturePort` trait.
- [x] Initialization of global system digital audio capture on macOS 12.3+ / 13+.
- [x] Conversion and sample rate alignment to 48kHz stereo float32 PCM blocks.
- [x] Transparent fallback to local `cpal` / generator in case of execution outside macOS or ungranted permissions.
- [x] Initialization and system audio capture test validated on the backend.

## Blocked by

- [01 — Workspace Scaffolding and Pure Loopback Audio Streaming](01-scaffolding-loopback-audio.md)
