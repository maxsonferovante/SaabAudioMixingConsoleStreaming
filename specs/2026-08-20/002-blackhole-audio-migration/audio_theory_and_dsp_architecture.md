# Audio Engineering Theory, DSP Mechanics & Technical Architecture

This document details the acoustic theory, mathematical foundations, and real-time DSP implementation decisions for the **Broadcast ITU-R BS.775 Multi-Channel Downmix Engine** and the **Dynamic High-Resolution Sample Rate Architecture** within `SaabAudioMixingConsoleStreaming`.

---

## 1. Multi-Channel Downmixing & ITU-R BS.775 Standard

### 1.1 The Acoustic & Psychoacoustic Challenge

When capturing multi-channel audio from a DAW (16 channels), a game engine (5.1 or 7.1 surround), or a video player, the signal contains discrete spatial channels meant for physical speakers positioned around the listener:
- **Left ($L$) & Right ($R$)**: Front stereo field.
- **Center ($C$)**: Monophonic anchor channel primarily containing dialogue, vocals, and centered solo instruments.
- **Low-Frequency Effects ($LFE$)**: Subwoofer channel (band-limited to $<120\text{Hz}$) for non-directional bass impacts.
- **Surround Left ($Ls$) & Surround Right ($Rs$)**: Ambient reverberation, side reflections, and rear directional cues.

The Android receiver outputs audio to an external stereo sound system or headphones via a **physical 3.5mm P2 (TRS) connection**, which has exactly two physical channels: Left and Right.

#### Why Naive Channel Summation Fails
1. **Acoustic Power Doubling (+6dB Voltage Gain / Comb Filtering)**:
   If the center channel $C$ is summed equally into both Left and Right without attenuation ($L' = L + C$, $R' = R + C$), the acoustic sound pressure level of the center signal increases by $+6\text{dB}$ when played through stereo speakers. This overpowers the front stereo mix, causing dialogue and vocals to drown out musical instrumentation and ambient effects.
2. **Digital Arithmetic Clipping (Inter-Sample Peaking)**:
   Summing 6 or 16 unweighted full-scale float signals easily exceeds the $[-1.0, +1.0]$ normalization ceiling, introducing harsh non-linear digital clipping and harmonic distortion.
3. **Channel Truncation Loss**:
   Simply taking channels 1 and 2 and discarding channels 3 through 16 results in complete loss of dialogue in 5.1 movies (since dialogue resides on channel 3/Center) and total loss of rear footsteps in competitive video games.

---

### 1.2 The ITU-R BS.775 Mathematical Formulation

To resolve these psychoacoustic issues, the International Telecommunication Union established recommendation **ITU-R BS.775** (*"Multichannel stereophonic sound system with or without accompanying picture"*).

ITU-R BS.775 specifies that when folding a $3/2$ surround configuration (5.1) into a $2/0$ stereo configuration, the center and surround channels must be attenuated by precisely **$-3\text{dB}$**:

$$\text{Attenuation Factor} = 10^{-3/20} = \frac{1}{\sqrt{2}} \approx 0.7071067811865475$$

In IEEE 754 float arithmetic, this corresponds to the constant `std::f32::consts::FRAC_1_SQRT_2`.

#### The Downmixing Matrix Equations:

$$L_{\text{out}} = L_{\text{in}} + \left(\frac{1}{\sqrt{2}}\right) C_{\text{in}} + \left(\frac{1}{\sqrt{2}}\right) Ls_{\text{in}}$$

$$R_{\text{out}} = R_{\text{in}} + \left(\frac{1}{\sqrt{2}}\right) C_{\text{in}} + \left(\frac{1}{\sqrt{2}}\right) Rs_{\text{in}}$$

#### Acoustic Energy Conservation:
- **Center Phantom Imaging**: Splitting the center signal $C$ between $L$ and $R$ with a $\frac{1}{\sqrt{2}}$ multiplier ensures that the total radiated acoustic power in the room remains constant:
  
  $$\text{Power}_{\text{total}} = \left(\frac{C}{\sqrt{2}}\right)^2 + \left(\frac{C}{\sqrt{2}}\right)^2 = \frac{C^2}{2} + \frac{C^2}{2} = C^2$$

  The perceived loudness of dialogue and vocals in the stereo fold-down matches the perceived loudness in a true 5.1 surround listening environment.
- **Surround Enclosure**: The rear surround energy ($Ls, Rs$) is mapped to the corresponding lateral stereo channels with $-3\text{dB}$ attenuation, preserving directional spatial cues without overpowering the front soundstage.
- **LFE Handling**: In accordance with broadcast mixing practice, the dedicated subwoofer channel ($LFE$) is excluded from direct stereo downmixing to prevent bass phase cancellation and amplifier saturation, as main stereo music tracks already contain full-range low-frequency content.

---

### 1.3 Implementation in `MacAudioCapture`

In [`crates/server/src/adapters/capture_macos.rs`](file:///Users/mferovante/Documents/workspace/AudioMixingConsoleStreaming/crates/server/src/adapters/capture_macos.rs), downmixing is implemented as a pure, zero-allocation function executed in the real-time audio thread:

```rust
use std::f32::consts::FRAC_1_SQRT_2;

#[inline(always)]
pub fn downmix_frame_to_stereo(frame: &[f32]) -> (f32, f32) {
    match frame.len() {
        0 => (0.0, 0.0),
        1 => (frame[0], frame[0]), // Mono duplication
        2 => (frame[0], frame[1]), // Bit-exact stereo passthrough
        6 => {
            // 5.1 Surround (Ch 0: L, Ch 1: R, Ch 2: C, Ch 3: LFE, Ch 4: Ls, Ch 5: Rs)
            let c = frame[2] * FRAC_1_SQRT_2;
            let l = frame[0] + c + frame[4] * FRAC_1_SQRT_2;
            let r = frame[1] + c + frame[5] * FRAC_1_SQRT_2;
            (l, r)
        }
        _ => {
            // 16-channel BlackHole DAW / Multitrack routing:
            // Checks if surround/center channels have active signal energy
            if frame.len() >= 6 && (frame[2].abs() > 0.001 || frame[4].abs() > 0.001) {
                let c = frame[2] * FRAC_1_SQRT_2;
                let l = frame[0] + c + frame[4] * FRAC_1_SQRT_2;
                let r = frame[1] + c + frame[5] * FRAC_1_SQRT_2;
                (l, r)
            } else {
                // Main Stereo Master Bus (Channels 1 & 2)
                (frame[0], frame[1])
            }
        }
    }
}
```

---

## 2. Dynamic High-Resolution Sample Rates & 5ms Windowing

### 2.1 The Sampling Theorem & High-Resolution Audio

The **Nyquist-Shannon Sampling Theorem** states that a continuous-time signal can be completely represented and reconstructed if the sampling frequency $f_s$ satisfies:

$$f_s > 2 \cdot f_{\text{max}}$$

- **44.1 kHz / 48 kHz (Standard Resolution)**: Captures frequencies up to $22.05\text{kHz}$ and $24\text{kHz}$, covering the full nominal range of human hearing ($20\text{Hz} - 20\text{kHz}$).
- **96 kHz / 192 kHz (High-Resolution Studio Master)**:
  - **Relaxed Anti-Aliasing Filters**: At $48\text{kHz}$, steep brick-wall anti-aliasing low-pass filters near $20\text{kHz}$ introduce phase non-linearities and time-domain pre-ringing. At $96\text{kHz}$ and $192\text{kHz}$, the Nyquist cutoff is pushed to $48\text{kHz}$ and $96\text{kHz}$, allowing gentler, phase-linear reconstruction filters well beyond human hearing.
  - **Temporal Precision**: At $192\text{kHz}$, time resolution between consecutive audio samples is:
    $$\Delta t = \frac{1}{192000\text{ Hz}} \approx 5.208\ \mu\text{s}$$
    (compared to $20.833\ \mu\text{s}$ at $48\text{kHz}$). This provides fine microsecond-level transient response for studio mastering and acoustic monitoring.

---

### 2.2 The 5ms Temporal Frame Window

Real-time audio streaming requires balancing two competing physical constraints:

1. **Latency Threshold (Psychoacoustics of the Haas Effect)**:
   - Delays $>15\text{ms}$ between visual events (screen actions, fader adjustments) and acoustic feedback are perceived as jarring lag.
   - For real-time mixing consoles and interactive monitoring, total end-to-end transport latency must remain **$<10\text{ms}$**.
2. **Network Protocol Overhead & Socket Context Switching**:
   - Emitting a UDP packet for every single audio sample would generate $192,000\text{ packets/second}$, overwhelming OS network stacks with interrupt processing.
   - A **5-millisecond buffer window ($\Delta T = 0.005\text{s}$)** provides an ideal equilibrium, emitting exactly **200 packets per second**.

#### Mathematical Calculation of Chunk Sizes ($N_{\text{frames}}$):

$$N_{\text{frames}} = f_s \cdot \Delta T = \frac{f_s \cdot 5}{1000}$$

| Sample Rate ($f_s$) | Nyquist Cutoff ($f_{\text{Nyq}}$) | Frame Count ($N_{\text{frames}}$) | Float32 Stereo Samples | Payload Size (Bytes) | Header + Payload (Total Bytes) |
| :---: | :---: | :---: | :---: | :---: | :---: |
| **44,100 Hz** | 22.05 kHz | 220 frames | 440 samples | 1,760 B | 1,788 B |
| **48,000 Hz** | 24.00 kHz | 240 frames | 480 samples | 1,920 B | 1,948 B |
| **88,200 Hz** | 44.10 kHz | 441 frames | 882 samples | 3,528 B | 3,556 B |
| **96,000 Hz** | 48.00 kHz | 480 frames | 960 samples | 3,840 B | 3,868 B |
| **176,400 Hz** | 88.20 kHz | 882 frames | 1,764 samples | 7,056 B | 7,084 B |
| **192,000 Hz** | 96.00 kHz | 960 frames | 1,920 samples | 7,680 B | 7,708 B |

Even at the highest studio rate ($192\text{kHz}$), the total datagram size ($7,708\text{ bytes}$) easily fits within our $65,536\text{ byte}$ socket buffers, preventing packet fragmentation on standard USB networks and Gigabit Wi-Fi.

---

### 2.3 Binary Header Protocol Specification (`protocol`)

Each packet transmitted over UDP or USB TCP contains an immutable 28-byte binary header followed by raw uncompressed IEEE 754 float32 PCM samples:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Magic: b"AMCS"                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Sequence Number (Bits 0..31)                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Sequence Number (Bits 32..63)                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Timestamp (Microseconds 0..31)              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Timestamp (Microseconds 32..63)             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Sample Rate (Hz, u32)                    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         Channels (u16)        |      Sample Count (u16)       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Format (u8)  | Reserved (u8) | Interleaved Float32 Audio ... |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.4 Android Hardware DAC Clock Synchronization

When the Android client receives packets:
1. `UdpAudioReceiver` decodes the 28-byte header and reads the exact `sample_rate` (e.g. 48000, 96000, 192000).
2. De-jittered float32 samples are pushed into a lock-free Single-Producer Single-Consumer (SPSC) ring buffer (`HeapRb<f32>`).
3. Google Oboe operates in **AAudio Exclusive Mode** (`SharingMode::Exclusive`, `PerformanceMode::LowLatency`).
4. The Android audio HAL syncs its DMA hardware interrupt callback directly with the incoming stream. If the hardware DAC supports high-resolution audio (common on modern Snapdragon DACs and external USB-C/3.5mm DAC dongles), playback occurs bit-exact without resampling; if the internal DAC is clocked at 48kHz, Oboe's built-in band-limited resampler performs transparent, phase-corrected rate conversion.

---

## 3. Summary of Architectural Advantages

| Metric / Feature | Previous Architecture (ScreenCaptureKit) | Current Architecture (BlackHole 16ch CoreAudio HAL) |
| :--- | :--- | :--- |
| **Driver Overhead** | Window Server video capture pipeline | Direct CoreAudio HAL loopback ($0\text{ms}$ driver latency) |
| **Visual Intrusiveness** | Persistent purple screen recording menu bar icon | None ($100\%$ background audio daemon) |
| **Speaker Isolation** | Echoes on Mac speakers & Android simultaneously | Audio routed exclusively to Android 3.5mm P2 jack |
| **Max Sample Rate** | Locked to 48,000 Hz | High-Resolution ($44.1\text{kHz}$ up to $192,000\text{Hz}$) |
| **Multi-Channel Feeds** | 2 channels only (truncates 5.1/7.1 content) | 16 channels with standard ITU-R BS.775 downmix |
| **Toolchain Dependencies** | Requires Swift runtime libraries (`libswiftCore`) | $100\%$ Pure Safe Rust (Zero Swift dependencies) |
