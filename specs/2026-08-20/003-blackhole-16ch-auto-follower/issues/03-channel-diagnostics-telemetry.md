# 03 — 16-Channel Diagnostic Telemetry & Active Channel Mapping

**What to build:** Granular telemetry and structured diagnostic logging in `crates/server` that computes RMS and Peak energy metrics per channel pair ($0\text{-}1, 2\text{-}3, 4\text{-}5, \dots, 14\text{-}15$) over 5ms accumulated audio frames, logging an active channel map when signal is present and emitting transition logs when the Auto-Follower switches drivers.

**Blocked by:** 02 — CoreAudio HAL Dynamic Auto-Follower & Hot-Migration

**Status:** done

- [x] Telemetry reports channel-pair signal levels when active audio is present.
- [x] Device migration events log the previous and newly activated CoreAudio device names and channel counts.
- [x] Zero performance degradation on real-time audio threads.
