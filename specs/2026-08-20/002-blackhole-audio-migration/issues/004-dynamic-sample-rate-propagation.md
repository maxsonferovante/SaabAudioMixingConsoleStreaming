# Issue 004: Dynamic High-Resolution Sample Rate Propagation & Android Oboe Sync

## What to build

Extract the active sample rate (44.1kHz, 48kHz, 96kHz, 192kHz) from the device input configuration, dynamically calculate chunk sizes (5ms frame windows: 240 frames at 48kHz, 480 frames at 96kHz, 960 frames at 192kHz), and encode the rate into the 28-byte `AudioPacketHeader`. Ensure the Android client receiver and Google Oboe AAudio engine configure their output streams cleanly for the incoming sample rate.

## Acceptance criteria

- [ ] CoreAudio stream dynamically reads hardware sample rate (e.g. 48000, 96000, 192000) and populates `AudioPacketHeader.sample_rate`.
- [ ] Android client audio receiver processes packets across sample rate spectrum without ring buffer underruns.
- [ ] No audio pitch shifting or drift observed during playback.

## Blocked by

- Issue 001: Purge ScreenCaptureKit & Implement Pure CoreAudio HAL Capture
