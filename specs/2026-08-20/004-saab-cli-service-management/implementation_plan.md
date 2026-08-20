# Implementation Plan: Unified `saab` CLI for Zero-Friction Operations & Homebrew Distribution

Create a unified, production-grade CLI tool (`saab`) that eliminates multi-terminal friction by providing a single command suite: `saab configure`, `saab start`, `saab stop`, `saab status`, `saab logs`, and `saab studio`, with automatic GitHub release asset downloads and service lifecycle management.

## User Review Required

> [!IMPORTANT]
> **No Emojis Policy**: Strictly enforced across all code, logs, CLI outputs, releases, and documentation.
> **Homebrew Formula Ready**: The `saab` binary is self-contained and downloads the Android `aarch64-linux-android` receiver binary from GitHub releases on demand, requiring no Android NDK on the user's Mac.

---

## Architecture & Commands Design

### CLI Command Suite (`crates/cli` -> `saab`)

```
saab <COMMAND>

Commands:
  configure  Interactive wizard to configure audio devices, network targets, and ADB settings
  start      Start macOS audio capture server and Android audio node in background services
  stop       Stop both macOS server and Android receiver services
  status     Display real-time service status, PIDs, active audio driver, and network health
  logs       Stream live logs (--server-mac or --device-android)
  studio     Launch the Dedicated Studio Touch Console (Iced GUI)
  help       Print help information
```

### Configuration Storage (`~/.config/saab/config.json`)

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

### Service Lifecycle & Process Management

1. **`saab configure`**:
   - Probes `cpal` to list installed CoreAudio HAL drivers (detecting `BlackHole 16ch` / `BlackHole 2ch`).
   - Probes `adb devices` and `ip -f inet addr show wlan0` to auto-detect attached Android devices and their Wi-Fi IPs.
   - Interactively confirms settings and saves to `~/.config/saab/config.json`.

2. **`saab start`**:
   - Checks if `server` or `client` is already running by process name; if running, notifies user and restarts cleanly.
   - Spawns macOS server in background as a daemon process, capturing stdout/stderr into `~/.config/saab/logs/server.log`.
   - Checks if Android device has the receiver binary at `/data/local/tmp/client`.
   - If missing, queries GitHub Releases API for `SaabAudioMixingConsoleStreaming` (e.g. `v0.3.1`), downloads the precompiled `client-aarch64-linux-android`, pushes to `/data/local/tmp/client`, and sets permissions (`chmod +x`).
   - Configures ADB ports (`adb forward tcp:48480 tcp:48480` and `adb reverse tcp:9001 tcp:9001`).
   - Spawns the Android audio daemon via `adb shell "nohup /data/local/tmp/client > /data/local/tmp/client.log 2>&1 &"`.
   - Prints connection summary with target IP, sample rate, and active CoreAudio driver.

3. **`saab stop`**:
   - Terminates the macOS server daemon process.
   - Executes `adb shell pkill -f /data/local/tmp/client` to terminate the Android receiver.
   - Cleans up PID locks.

4. **`saab status`**:
   - Inspects macOS server process state, uptime, and memory.
   - Probes ADB to verify if the Android audio daemon is active.
   - Shows active audio stream parameters and WebSocket connection state.

5. **`saab logs`**:
   - Flag `--server-mac`: Streams `~/.config/saab/logs/server.log` (`tail -f` style in pure Rust).
   - Flag `--device-android`: Streams logs directly from Android via `adb shell tail -f /data/local/tmp/client.log`.

6. **`saab studio`**:
   - Launches the Iced GUI Studio Touch Console window in foreground, connecting to `ws://127.0.0.1:9001`.

---

## Proposed Changes

### Workspace & New Crate

#### [NEW] [crates/cli/Cargo.toml](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/Cargo.toml)
CLI crate dependencies: `clap` (derive), `serde`, `serde_json`, `inquire` (or `dialoguer` for interactive prompts), `dirs`, `tokio`, `anyhow`, `tracing`, `tracing-subscriber`, `reqwest` (for GitHub release download), `cpal`, `client`, `server`, `protocol`, `audio_core`.

#### [NEW] [crates/cli/src/main.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/main.rs)
CLI entrypoint parsing commands via `clap`.

#### [NEW] [crates/cli/src/commands/mod.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/commands/mod.rs)
Subcommand router.

#### [NEW] [crates/cli/src/commands/configure.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/commands/configure.rs)
Interactive auto-discovery and configuration generator.

#### [NEW] [crates/cli/src/commands/start.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/commands/start.rs)
Background daemon orchestrator for macOS server and Android node, with automatic binary download from GitHub Releases.

#### [NEW] [crates/cli/src/commands/stop.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/commands/stop.rs)
Clean shutdown handler for macOS and Android services.

#### [NEW] [crates/cli/src/commands/status.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/commands/status.rs)
Status inspector for server, ADB node, and audio streams.

#### [NEW] [crates/cli/src/commands/logs.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/commands/logs.rs)
Log tailing implementation for macOS and Android.

#### [NEW] [crates/cli/src/commands/studio.rs](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/cli/src/commands/studio.rs)
Invokes `ConsoleApp::run` from `client::ui`.

#### [MODIFY] [Cargo.toml](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/Cargo.toml)
Add `crates/cli` to workspace members and configure `default-run = "saab"`.

---

## Verification Plan

### Automated Tests
- Run `cargo test --workspace` ensuring all crates, domain tests, loopback tests, and CLI unit tests pass.
- Run `cargo clippy --workspace -- -D warnings`.

### Manual Verification
- Test `cargo run --bin saab -- configure` generating `~/.config/saab/config.json`.
- Test `cargo run --bin saab -- start` verifying both macOS server and Android client start in background.
- Test `cargo run --bin saab -- status` inspecting active PIDs and stream state.
- Test `cargo run --bin saab -- logs --server-mac` and `cargo run --bin saab -- logs --device-android`.
- Test `cargo run --bin saab -- studio` opening the Iced touch console.
- Test `cargo run --bin saab -- stop` cleanly stopping all services.
