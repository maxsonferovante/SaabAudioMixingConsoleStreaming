## Parent

#22 (Spec 004: Unified saab CLI & Service Management Engine)

## What to build

Implement real-time status diagnostics (`saab status`) and live log streaming (`saab logs`). `saab status` displays active process states, PIDs, active CoreAudio drivers, network addresses, sample rate, and connection latency. `saab logs --server-mac` tails the local server log (`~/.config/saab/logs/server.log`), and `saab logs --device-android` streams stdout/stderr from `/data/local/tmp/client.log` on the Android device via ADB.

## Acceptance criteria

- [x] `saab status` reports PID, uptime, status (RUNNING/STOPPED), target IP, sample rate, and driver name for both macOS server and Android node.
- [x] `saab logs --server-mac` streams server log lines in real time to the terminal.
- [x] `saab logs --device-android` streams Android client logs via ADB in real time.
- [x] If no flags are provided, `saab logs` displays helpful usage info or defaults to server logs.
- [x] Output formatting is clean, structured, and strictly without emojis.

## Blocked by

- #24 (Process Supervisor, Daemon Management & Clean Lifecycle)
