## Parent

#22 (Spec 004: Unified saab CLI & Service Management Engine)

## What to build

Implement the primary CLI crate (`crates/cli`) producing the standalone `saab` binary (with alias `saab-audio-console`). The binary implements a type-safe `clap` router and the interactive `saab configure` command, which auto-probes CoreAudio HAL loopback devices (`BlackHole 16ch`, `BlackHole 2ch`) and connected ADB targets, interactively confirms configuration settings, and writes them to `~/.config/saab/config.json`.

## Acceptance criteria

- [x] Crate `crates/cli` is registered in root `Cargo.toml` workspace members and builds binary `saab`.
- [x] Running `saab --help` outputs all standard subcommands: `configure`, `start`, `stop`, `status`, `logs`, `studio`.
- [x] `saab configure` probes available audio input devices using `cpal` and detects ADB devices via `adb devices`.
- [x] Configuration is written in valid JSON format to `~/.config/saab/config.json` with sane defaults for missing fields.
- [x] Unit tests validate configuration serialization/deserialization and CLI argument parsing with zero warnings.

## Blocked by

None - can start immediately
