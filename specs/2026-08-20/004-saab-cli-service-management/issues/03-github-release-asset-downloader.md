## Parent

#22 (Spec 004: Unified saab CLI & Service Management Engine)

## What to build

Implement the automated asset fetcher and deployment subsystem. When `saab start` runs and `/data/local/tmp/client` is missing on the connected Android device, the downloader queries the GitHub Releases API for `maxsonferovante/SaabAudioMixingConsoleStreaming` (matching current CLI version), downloads `client-aarch64-linux-android` to `~/.cache/saab/bin/`, pushes it to `/data/local/tmp/client` with `libc++_shared.so`, and grants execution permissions. If a local release binary is present in `target/aarch64-linux-android/release/client`, it uses the local artifact directly without hitting the network.

## Acceptance criteria

- [ ] Downloader checks local workspace target build first before reaching out to GitHub.
- [ ] If local binary is absent, queries GitHub Releases API and downloads the precompiled `aarch64-linux-android` client binary into `~/.cache/saab/bin/client`.
- [ ] Automatically pushes the client binary and `libc++_shared.so` to `/data/local/tmp/` via ADB.
- [ ] Sets executable permissions (`chmod +x /data/local/tmp/client`).
- [ ] Users on macOS without Android NDK can start the Android receiver with a single command.

## Blocked by

- #23 (CLI Scaffolding, Subcommand Parser & Configuration System)
- #24 (Process Supervisor, Daemon Management & Clean Lifecycle)
