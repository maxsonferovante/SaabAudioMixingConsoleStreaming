# Tickets: BlackHole 16ch CoreAudio Virtual Driver Migration

## Proposed Vertical Slices (Tracer Bullets)

### Ticket 1: Purge ScreenCaptureKit & Implement Pure CoreAudio HAL Capture
- **Type**: AFK
- **Blocked by**: None (Can start immediately)
- **User stories covered**: Story 1, Story 8
- **What to build**:
  - Remove `screencapturekit = "8.0.1"` from `crates/server/Cargo.toml`.
  - Delete `crates/server/build.rs` Swift runtime linking script.
  - Implement base CoreAudio HAL audio capture in `crates/server/src/adapters/capture_macos.rs` using `cpal`.
- **Acceptance criteria**:
  - [ ] Server compiles in 100% pure Rust without invoking Swift toolchains or linking `libswiftCore`.
  - [ ] macOS screen recording indicator is never triggered when server starts audio capture.
  - [ ] Workspace compiles with zero compiler warnings.

---

### Ticket 2: Intelligent Device Auto-Discovery & Diagnostic Guidance
- **Type**: AFK
- **Blocked by**: Ticket 1
- **User stories covered**: Story 3, Story 4
- **What to build**:
  - Implement prioritized device discovery in `MacAudioCapture`:
    1. CLI argument `--device <name>` or `AUDIO_DEVICE` environment variable.
    2. Exact match on `BlackHole 16ch` (Project Default).
    3. Exact match on `BlackHole 2ch` or `BlackHole 64ch`.
    4. Substring match on `blackhole` / `loopback`.
    5. Fallback to default input device with informative installation guidance.
  - Log copy-pasteable Homebrew installation commands (`brew install blackhole-16ch`) when no virtual device is detected.
- **Acceptance criteria**:
  - [ ] Server automatically binds to `BlackHole 16ch` when available.
  - [ ] Server binds to `BlackHole 2ch` or `BlackHole 64ch` if 16ch is absent.
  - [ ] If no BlackHole device is detected, server logs clear `brew install blackhole-16ch` instructions and continues gracefully on default input.

---

### Ticket 3: ITU-R BS.775 Broadcast Downmix Engine for Multi-Channel Inputs
- **Type**: AFK
- **Blocked by**: Ticket 1
- **User stories covered**: Story 6
- **What to build**:
  - Implement standard broadcast ITU-R BS.775 weighted downmixing for multi-channel input streams (5.1, 7.1, 16ch):
    $$L = L + 0.7071 \cdot C + 0.7071 \cdot Ls$$
    $$R = R + 0.7071 \cdot C + 0.7071 \cdot Rs$$
  - Maintain bit-exact 1:1 passthrough for 2-channel stereo sources.
  - Add unit tests verifying downmixing mathematical correctness and zero clipping.
- **Acceptance criteria**:
  - [ ] 2-channel stereo inputs pass through untouched.
  - [ ] 6-channel (5.1) and 16-channel inputs are downmixed to stereo preserving center dialogue and surround cues.
  - [ ] Unit tests in `capture_macos.rs` validate downmix calculations.

---

### Ticket 4: Dynamic High-Resolution Sample Rate Propagation & Android Oboe Sync
- **Type**: AFK
- **Blocked by**: Ticket 1
- **User stories covered**: Story 5, Story 7
- **What to build**:
  - Extract active sample rate (44.1kHz, 48kHz, 96kHz, 192kHz) from BlackHole stream configuration.
  - Dynamically size audio processing chunks (5ms frame windows: 240 frames at 48kHz, 480 frames at 96kHz, 960 frames at 192kHz).
  - Transmit exact sample rate in `AudioPacketHeader` (28 bytes).
  - Verify Android client and Oboe AAudio engine handle incoming rate seamlessly.
- **Acceptance criteria**:
  - [ ] Sample rate dynamically queried from CoreAudio device and written into `AudioPacketHeader.sample_rate`.
  - [ ] Android client correctly decodes sample packets across 44.1kHz to 192kHz.
  - [ ] Zero audio pitch drift or buffering stalls.

---

### Ticket 5: Local Loopback Integration Test Suite & Workspace CI Validation
- **Type**: AFK
- **Blocked by**: Ticket 2, Ticket 3, Ticket 4
- **User stories covered**: Story 9, Story 10
- **What to build**:
  - Create integration test verifying end-to-end audio streaming and command loop over `127.0.0.1`.
  - Update `README.md` with BlackHole 16ch setup instructions, architecture diagram, and prerequisites.
  - Run formatting, clippy, and all workspace tests.
- **Acceptance criteria**:
  - [ ] `cargo test --workspace` passes 100% of tests.
  - [ ] `cargo clippy --workspace -- -D warnings` reports 0 warnings.
  - [ ] `cargo fmt --check` succeeds.
  - [ ] `README.md` accurately describes BlackHole 16ch setup.
