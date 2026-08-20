# PRD 004: Unified `saab` CLI & Service Management for Zero-Friction Operations

## Problem Statement

Currently, running and operating the audio mixing console streaming system requires opening and managing three separate, concurrent terminal windows:
1. One terminal running the macOS audio capture server (`cargo run --bin server -- <ANDROID_IP>:48480`).
2. A second terminal executing ADB and keeping an attached interactive shell session for the Android audio receiver (`./scripts/build_android.sh`).
3. A third terminal to launch the desktop touch console interface (`cargo run --bin client`).

This multi-terminal setup creates significant friction, is error-prone (e.g. port conflicts, hanging terminal processes, lost IP addresses), and discourages seamless daily adoption. Furthermore, preparing the software for general distribution via package managers such as Homebrew (`brew install saab`) requires a single, unified command-line interface that manages background service lifecycles, auto-configures network/audio settings into persistent configuration files, automatically downloads Android binaries without requiring local Android NDK installations, and offers dedicated log inspection commands.

## Solution

Build a unified, production-ready command-line tool named `saab` (with alias `saab-audio-console`) structured as a dedicated workspace crate (`crates/cli`):

1. **Interactive Configuration Wizard (`saab configure`)**: An auto-discovery setup command that probes CoreAudio HAL drivers (detecting `BlackHole 16ch` and `BlackHole 2ch`) and connected ADB devices (USB and Wi-Fi), asking only the necessary questions to save a persistent `config.json` in standard user configuration directories (`~/.config/saab/config.json`).
2. **One-Command Background Service Orchestration (`saab start` / `saab stop`)**:
   - Launches the macOS CoreAudio capture server as a managed background service with automatic process detection and restart safeguards.
   - Probes the Android target device via ADB; if the receiver binary is missing, automatically downloads the precompiled `aarch64-linux-android` binary directly from the matching GitHub Release without requiring Android NDK or build tools on the host Mac.
   - Configures ADB port rules (`adb forward tcp:48480 tcp:48480` and `adb reverse tcp:9001 tcp:9001`) and launches the Android audio node as a background daemon process.
   - `saab stop` cleanly halts both macOS and Android services by process name.
3. **Live Service Status & Diagnostics (`saab status`)**: Displays real-time process states, PIDs, active CoreAudio drivers, network addresses, sample rates, and connectivity health.
4. **Dedicated Log Inspection (`saab logs`)**: Streams real-time stdout and stderr logs from either the macOS server (`--server-mac`) or the remote Android receiver (`--device-android`).
5. **Standalone Touch Console Launcher (`saab studio`)**: Opens the Dedicated Studio Touch Console (Iced GUI) in a detached desktop window connected to the local WebSocket backend.
6. **Homebrew Distribution Compatibility**: Packaged so that end-users installing via `brew install saab` can immediately configure and stream audio out of the box with zero external SDK dependencies.

---

## User Stories

1. As a live streamer, I want to start my entire audio streaming system with a single `saab start` command, so that I do not have to open and manage three separate terminal windows.
2. As a content creator, I want `saab configure` to automatically scan and detect my BlackHole 16ch driver and Android device on my local Wi-Fi network, so that I do not have to look up network IP addresses or driver names manually.
3. As a user, I want `saab configure` to persist my settings into `~/.config/saab/config.json`, so that I only have to run the setup wizard once.
4. As a user who already has a server or client running, I want `saab start` to detect existing active processes by name, notify me, and restart them cleanly without hanging ports or leaving zombie processes.
5. As an audio professional, I want `saab start` to launch the macOS audio server in the background as a daemon service, so that my terminal session remains immediately free for other tasks.
6. As a macOS user installing the tool via Homebrew (`brew install saab`), I want the CLI to automatically fetch the matching `aarch64-linux-android` client binary from GitHub Releases when deploying to my phone, so that I do not need to install the Android NDK, Rust target, or `cargo-ndk` on my Mac.
7. As a mobile listener, I want `saab start` to configure ADB port forwarding and start the Android audio daemon in the background via ADB, so that the phone starts playing audio through the 3.5mm P2 jack immediately.
8. As a user finishing my broadcast, I want `saab stop` to terminate both the macOS server daemon and the Android audio receiver cleanly with a single command.
9. As a system administrator, I want `saab status` to display the active PIDs, uptime, memory, network latency, sample rate, and active CoreAudio device for both macOS and Android nodes.
10. As a developer troubleshooting audio issues, I want `saab logs --server-mac` to stream live stdout and stderr from the macOS server, so that I can monitor buffer sizes, downmixer peaks, and CoreAudio driver migration events.
11. As a developer troubleshooting mobile playback, I want `saab logs --device-android` to stream live logs from the Android Oboe AAudio receiver via ADB, so that I can inspect hardware buffer underruns and sample rate adaptations.
12. As a tactile mixing console user, I want `saab studio` to launch the Dedicated Studio Touch Console (Iced GUI) in a dedicated window, so that I can access my master volume fader and 60fps stereo VU meters on demand.
13. As a Homebrew user, I want the CLI binary to be named `saab` with zero external runtime dependencies, so that command execution is concise and memorable.
14. As an open-source user, I want all CLI commands, logs, and outputs to adhere to a clean, professional aesthetic without any emojis, so that terminal outputs remain readable across all shell environments.

---

## Implementation Decisions

### Workspace Architecture & CLI Crate
- Create a dedicated workspace crate `crates/cli` producing the primary executable `saab` (with alias `saab-audio-console`).
- Use `clap` with derive macros for type-safe subcommand routing (`configure`, `start`, `stop`, `status`, `logs`, `studio`).
- Use `inquire` or `dialoguer` for ergonomic, accessible interactive CLI configuration prompts.

### Service Daemon Management
- **macOS Server Lifecycle**:
  - Spawned as a background child process detached from the current shell session.
  - Process tracking using standard PID files and process table verification (`pgrep -f server` / `saab-server`).
  - Standard output and standard error redirected to `~/.config/saab/logs/server.log`.
- **Android Node Lifecycle**:
  - Process execution managed via ADB shell invocation (`adb shell "nohup /data/local/tmp/client > /data/local/tmp/client.log 2>&1 &"`).
  - Process verification via `adb shell pgrep -f /data/local/tmp/client`.
  - Termination handled via `adb shell pkill -f /data/local/tmp/client`.
  - Port forwarding automatically orchestrated on launch (`adb forward tcp:48480 tcp:48480` and `adb reverse tcp:9001 tcp:9001`).

### Automatic Asset Provisioning (GitHub Releases Downloader)
- Implement an automated asset downloader using `reqwest` that queries the GitHub Releases API for `maxsonferovante/SaabAudioMixingConsoleStreaming` matching the current workspace version (e.g. `v0.3.1`).
- Downloads `client-aarch64-linux-android` into a local cache directory (`~/.cache/saab/`) and transfers it to the connected device via `adb push` along with `libc++_shared.so`.
- If a local release binary is present (during development in `target/aarch64-linux-android/release/client`), the CLI prioritizes the local artifact without hitting network endpoints.

### Touch Console Execution (`saab studio`)
- Invokes the existing `ConsoleApp::run` implementation from `client::ui` directly within the GUI thread, preserving dark studio themes, 60fps stereo VU meters, and WebSocket auto-reconnect logic.

---

## Testing Decisions

- **CLI Parsing Unit Tests**: Verify that all subcommands (`configure`, `start`, `stop`, `status`, `logs`, `studio`) and flags parse arguments and options accurately.
- **Config Serialization Tests**: Validate that configuration structs serialize and deserialize to and from JSON format, ensuring backwards-compatible defaults for missing keys.
- **Process Supervisor Invariance Tests**: Ensure that starting an already running service triggers graceful restart notifications rather than resource leaks or panic states.
- **Zero-I/O Mocking**: Test configuration generation and validation in memory with isolated mock directories.

---

## Out of Scope

- Native macOS GUI status menu bar icon (CLI and Studio Touch Console are the primary interfaces for this milestone).
- Automatic Wi-Fi password configuration via Bluetooth/BLE (Wi-Fi connectivity is configured using standard Android OS settings).
- Supporting non-Android mobile platforms (iOS AAudio is not applicable).

---

## Further Notes

- Configuration directory standard: `~/.config/saab/` (or platform equivalent via `dirs::config_dir()`).
- Log files standard: `~/.config/saab/logs/server.log` and `/data/local/tmp/client.log`.
- Strict policy: Absolutely no emojis across all code, logs, and CLI output streams.
