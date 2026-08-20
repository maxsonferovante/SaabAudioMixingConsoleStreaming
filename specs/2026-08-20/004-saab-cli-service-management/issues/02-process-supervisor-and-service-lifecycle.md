## Parent

#22 (Spec 004: Unified saab CLI & Service Management Engine)

## What to build

Implement the service lifecycle and process supervisor engine for `saab start` and `saab stop`. `saab start` runs the macOS audio capture server in the background as a detached daemon process (logging to `~/.config/saab/logs/server.log`), sets up ADB port forwarding (`adb forward tcp:48480 tcp:48480` and `adb reverse tcp:9001 tcp:9001`), and starts the Android audio daemon via ADB in the background. `saab stop` cleanly halts both services by process name and clears PID locks. If services are already alive on `start`, the supervisor notifies the user and restarts them gracefully.

## Acceptance criteria

- [x] `saab start` launches macOS server as a detached background daemon and writes PID to `~/.config/saab/pids/server.pid`.
- [x] `saab start` executes ADB port forward and reverse commands automatically.
- [x] `saab start` detects running instances by process name, logs a restart notification, and gracefully recycles the processes without hanging.
- [x] `saab stop` terminates both macOS server and Android receiver processes cleanly.
- [x] Standard output and standard error from server are appended to `~/.config/saab/logs/server.log`.

## Blocked by

- #23 (CLI Scaffolding, Subcommand Parser & Configuration System)
