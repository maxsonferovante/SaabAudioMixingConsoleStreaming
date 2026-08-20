## What to build

A native macOS audio capture adapter in the backend (`server`) that integrates with Apple's native ScreenCaptureKit (`SCStream`) and CoreAudio APIs to capture all digital audio output emitted by the operating system and active applications (such as Spotify, browsers, games, calls) without requiring third-party virtual drivers (such as BlackHole or Perssua), feeding the real-time streaming pipeline.

## Native System Capture Architecture (ScreenCaptureKit)

### Option 1: Native Capture via Apple ScreenCaptureKit (Zero Driver)
- **Mechanism**: Utilizes Apple's native ScreenCaptureKit framework (`SCStream` with `capturesAudio = true`) introduced in macOS 13+ to capture digital system audio output directly from the OS compositor.
- **Advantages**: 100% native to macOS, bit-exact 48kHz stereo float32 stream, zero third-party drivers or kernel extensions required.
- **Permissions**: Requires *Screen & System Audio Recording* permission in *System Settings -> Privacy & Security*.
- **Fallback**: Transparently falls back to physical input / loopback devices via `cpal` if system recording permission is not yet granted or if an explicit hardware input device is requested via CLI.

## Acceptance criteria

- [x] Implementation of the audio capture adapter (`MacAudioCapture`) implementing the `AudioCapturePort` trait.
- [x] Native driverless system digital audio capture via `ScreenCaptureKit` (`SCStream`) on macOS 13+.
- [x] Extraction of raw PCM float32 samples from `CMSampleBuffer` / `AudioBufferList` and sample rate alignment to 48kHz stereo blocks.
- [x] Automatic fallback to local `cpal` hardware/virtual audio input when ScreenCaptureKit is unavailable.
- [x] Initialization and system audio capture test validated on the backend.

## Blocked by

- [01 — Workspace Scaffolding and Pure Loopback Audio Streaming](01-scaffolding-loopback-audio.md)
