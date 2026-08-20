## Parent

#22 (Spec 004: Unified saab CLI & Service Management Engine)

## What to build

Implement the `saab studio` subcomponent to launch the Dedicated Studio Touch Console (Iced GUI) in a detached desktop window connected to `ws://127.0.0.1:9001`. Run end-to-end integration testing across the entire workspace (`cargo test --workspace`), verify strict linter compliance (`cargo clippy --workspace -- -D warnings`), document the Homebrew installation formula in `README.md`, and validate the complete zero-friction workflow from `saab configure` to `saab start`, `saab studio`, and `saab stop`.

## Acceptance criteria

- [x] `saab studio` launches the Iced GUI ConsoleApp in foreground or detached window without needing `cargo run --bin client`.
- [x] The studio touch console connects cleanly to the background server WebSocket on `ws://127.0.0.1:9001` and displays `[ONLINE]`.
- [x] End-to-end integration tests pass on the entire workspace with zero failures (`cargo test --workspace`).
- [x] Strict clippy linter passes with zero warnings (`cargo clippy --workspace -- -D warnings`).
- [x] Documentation for Homebrew installation and single-command workflow is updated in `README.md`.

## Blocked by

- #23 (CLI Scaffolding, Subcommand Parser & Configuration System)
- #24 (Process Supervisor, Daemon Management & Clean Lifecycle)
- #26 (Real-Time Diagnostic Status & Log Streamers)
