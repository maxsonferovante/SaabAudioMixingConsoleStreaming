# Issue 005: Integration Testing, Workspace CI Validation & Documentation

## What to build

Implement automated integration tests validating loopback packet streaming between server and client over local sockets. Update `README.md` to document the BlackHole 16ch driverless architecture, Homebrew installation commands, and system audio routing. Execute strict workspace linter and test validation.

## Acceptance criteria

- [ ] `cargo test --workspace` passes 100% of unit and integration tests.
- [ ] `cargo clippy --workspace -- -D warnings` completes with zero warnings.
- [ ] `cargo fmt --check` succeeds.
- [ ] `README.md` documentation reflects `BlackHole 16ch` as the primary capture driver.

## Blocked by

- Issue 002: BlackHole 16ch Auto-Discovery & Diagnostic Guidance
- Issue 003: ITU-R BS.775 Broadcast Downmix Engine for Multi-Channel Inputs
- Issue 004: Dynamic High-Resolution Sample Rate Propagation & Android Oboe Sync
