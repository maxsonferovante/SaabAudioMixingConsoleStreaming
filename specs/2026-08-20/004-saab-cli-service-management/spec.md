# Technical Specification 004: Unified `saab` CLI & Service Management Engine

## Problem Statement

Running the audio streaming ecosystem currently necessitates three concurrently active terminal windows (macOS audio capture server, ADB Android receiver session, and the touch console GUI). This causes operational fatigue, is prone to port conflicts or forgotten network endpoints, and blocks streamlined package distribution via Homebrew (`brew install saab`).

## Solution

A unified Rust-based CLI binary named `saab` (with backward-compatible alias `saab-audio-console`) structured as a dedicated workspace member (`crates/cli`). The CLI delivers:
1. An interactive auto-discovery configuration wizard (`saab configure`) generating persistent JSON configuration (`~/.config/saab/config.json`).
2. Automated background service orchestration (`saab start` and `saab stop`) managing the macOS server daemon and the Android ADB receiver node.
3. Automated GitHub Release binary downloads for the Android `aarch64-linux-android` client asset when not present locally.
4. Real-time log streaming (`saab logs --server-mac` / `saab logs --device-android`).
5. Real-time status diagnostics (`saab status`).
6. A dedicated launcher for the Studio Touch Console (`saab studio`).

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

### 1. Unified Command Structure & Subcommands
The CLI is built using `clap` (derive parser) providing the following command tree:

```
saab <SUBCOMMAND>

SUBCOMMANDS:
  configure     Interactive setup wizard for audio hardware, network IP, and ADB routing
  start         Orchestrate macOS server and Android receiver in background services
  stop          Cleanly stop all active streaming services
  status        Inspect real-time service health, PIDs, active audio driver, and latency
  logs          Stream stdout/stderr logs (--server-mac or --device-android)
  studio        Launch the Iced Studio Touch Console desktop GUI window
```

### 2. Configuration Schema (`~/.config/saab/config.json`)
The configuration is stored in standard XDG / OS user directories using the `dirs` crate:

```json
{
  "version": "1.0",
  "audio": {
    "device_name": "BlackHole 16ch",
    "sample_rate": 48000
  },
  "network": {
    "mode": "wifi",
    "android_ip": "192.168.15.5",
    "audio_port": 48480,
    "ws_port": 9001
  },
  "adb": {
    "target_device": "auto",
    "auto_reverse_port": true
  }
}
```

### 3. Background Service Lifecycle & Process Management
- **Process Identification**: Processes are tracked by process name (`saab-server` / `server` on macOS and `/data/local/tmp/client` on Android) and managed via PID lockfiles in `~/.config/saab/pids/`.
- **Restart Idempotency**: Running `saab start` when services are already alive detects existing PIDs, gracefully terminates previous instances (`SIGTERM` / `pkill`), clears stale socket binds, and starts fresh instances.
- **Log Redirection**:
  - macOS Server logs: Appended directly to `~/.config/saab/logs/server.log`.
  - Android Client logs: Appended to `/data/local/tmp/client.log` on the device and streamed via ADB.

### 4. Automated Asset Downloader for GitHub Releases
- When `saab start` runs and `/data/local/tmp/client` is missing from the connected Android device:
  1. Checks local workspace build at `target/aarch64-linux-android/release/client`.
  2. If absent, queries GitHub Releases API (`https://api.github.com/repos/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/latest` or matching tag).
  3. Downloads the `client-aarch64-linux-android` prebuilt binary into `~/.cache/saab/bin/client`.
  4. Pushes the binary and `libc++_shared.so` to `/data/local/tmp/` via ADB and sets execute permissions (`chmod +x`).

### 5. ADB Routing Enforcement
- Automatically executes:
  - `adb forward tcp:48480 tcp:48480` (for USB wire audio streaming)
  - `adb reverse tcp:9001 tcp:9001` (for reverse WebSocket telemetry synchronization without hijacking Mac localhost)

---

## Testing Decisions

### 1. CLI Parsing & Argument Validation Tests
- Unit tests in `crates/cli/src/tests/` validating argument combinations, flags (`--server-mac`, `--device-android`), default fallback parameters, and error message formatting.

### 2. Configuration Serialization Invariance
- Round-trip JSON unit tests verifying that all `Config` structs serialize, deserialize, and populate sane defaults when partial JSON configurations are loaded.

### 3. Process Supervisor Tests
- Verification of PID acquisition, status probing, and clean termination signals in mock process environments.

### 4. Zero Warnings Standard
- Full workspace verification passing `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`.

---

## Out of Scope

- Menu bar tray icons on macOS (GUI is handled via `saab studio`).
- Wi-Fi credential provisioning over Bluetooth (Wi-Fi network setup is handled via Android system settings).
- Supporting non-Android mobile operating systems.

---

## Further Notes

- Distribution target: Homebrew Formula (`brew install saab` / `brew tap maxsonferovante/saab && brew install saab`).
- Formatting standard: Strictly zero emojis across all outputs, logs, and documentation.
