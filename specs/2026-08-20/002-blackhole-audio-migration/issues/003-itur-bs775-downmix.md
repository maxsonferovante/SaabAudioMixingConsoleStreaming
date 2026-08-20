# Issue 003: ITU-R BS.775 Broadcast Downmix Engine for Multi-Channel Inputs

## What to build

Implement mathematically accurate ITU-R BS.775 broadcast surround downmixing inside the audio capture callback. For 2-channel inputs, pass stereo samples through with bit-exact fidelity and zero overhead. For multi-channel inputs (5.1, 7.1, 16-channel DAWs), apply center ($C$) and surround ($Ls$, $Rs$) downmixing ($L = L + 0.7071 \cdot C + 0.7071 \cdot Ls$, $R = R + 0.7071 \cdot C + 0.7071 \cdot Rs$) normalized to avoid digital clipping.

## Acceptance criteria

- [ ] 2-channel stereo sources pass directly to Left and Right channels with zero computational alteration.
- [ ] 6-channel (5.1) and 16-channel sources downmix to stereo preserving center dialogue and rear ambient channels.
- [ ] Downmix calculation unit tests in `crates/server` validate equations and assert no arithmetic overflow.

## Blocked by

- Issue 001: Purge ScreenCaptureKit & Implement Pure CoreAudio HAL Capture
