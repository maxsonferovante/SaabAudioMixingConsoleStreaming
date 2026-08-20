# Issue 001: Purge ScreenCaptureKit & Implement Pure CoreAudio HAL Capture

## What to build

Migrate the macOS audio capture pipeline from Apple ScreenCaptureKit to pure macOS CoreAudio HAL loopback capture. Eliminate the `screencapturekit` crate and delete the `build.rs` Swift toolchain linking script. Capture real-time system audio in safe, native Rust with zero screen recording indicators, zero display compositor overhead, and zero additional driver latency.

## Acceptance criteria

- [ ] All `screencapturekit` dependencies removed from `crates/server/Cargo.toml`.
- [ ] `crates/server/build.rs` deleted; server compiles in 100% pure safe Rust.
- [ ] Starting macOS audio capture does not trigger the macOS screen recording menu bar indicator.
- [ ] Server builds cleanly with zero compiler warnings.

## Blocked by

- None - can start immediately
