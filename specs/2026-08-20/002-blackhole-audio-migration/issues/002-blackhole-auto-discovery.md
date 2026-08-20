# Issue 002: BlackHole 16ch Auto-Discovery & Diagnostic Guidance

## What to build

Implement prioritized device auto-discovery in the CoreAudio capture adapter that scans available input devices and automatically binds to `BlackHole 16ch` (Project Default), with fallback to `BlackHole 2ch` and `BlackHole 64ch`, or CLI/environment overrides. If no BlackHole device is detected on macOS, display explicit, copy-pasteable Homebrew installation commands (`brew install blackhole-16ch`) and continue gracefully using the default input device.

## Acceptance criteria

- [ ] Automatic device resolution scans and selects `BlackHole 16ch` by default when present.
- [ ] Alternative variants (`BlackHole 2ch`, `BlackHole 64ch`) are selected if 16ch is absent.
- [ ] Command-line argument `--device <name>` and `AUDIO_DEVICE` environment variable override auto-discovery.
- [ ] Missing driver diagnostic outputs `brew install blackhole-16ch` instructions without crashing the server.

## Blocked by

- Issue 001: Purge ScreenCaptureKit & Implement Pure CoreAudio HAL Capture
