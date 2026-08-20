# 01 — Smart 16-Channel Downmixer & Anti-Clipping Soft Limiter

**What to build:** An acoustic multi-channel downmixing algorithm in the macOS server backend that maps even channels ($0, 2, 4, \dots$) to Left and odd channels ($1, 3, 5, \dots$) to Right, applies ITU-R BS.775 $-3\text{dB}$ power scaling ($\frac{1}{\sqrt{2}} \approx 0.70710678$) to center and secondary DAW channels, and passes the resulting stereo signals through a hyperbolic tangent ($\tanh$) soft limiter to guarantee zero harsh digital clipping under heavy multi-track summing.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] `downmix_frame_to_stereo` maps even channels to Left and odd channels to Right.
- [x] Center channel (Ch 2) and secondary pairs (Ch 4-5, 6-7, etc.) are attenuated by $-3\text{dB}$.
- [x] Multi-channel signal sums pass through $\tanh$ soft limiter, guaranteeing $|L| \le 1.0$ and $|R| \le 1.0$.
- [x] In-memory DSP unit tests for 2-channel passthrough, secondary DAW bus routing (channels 2 & 3), surround fold-down, and full 16-channel saturation.
