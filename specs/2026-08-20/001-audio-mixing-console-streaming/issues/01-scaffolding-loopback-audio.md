## What to build

A structured Cargo workspace with Clean Architecture (`protocol`, `core`, `server`, `client`) where the backend server captures raw audio from the input source, encapsulates pure uncompressed audio samples (raw PCM float32 stereo at 48kHz) in UDP datagrams without altering signal values, and streams in real-time to the client, which consumes and reproduces the audio with bit-exact integrity through a lock-free circular ring buffer (`ringbuf`), verified by a loopback integration test.

## Acceptance criteria

- [x] Root Cargo Workspace configured with `protocol`, `core`, `server`, and `client` crates.
- [x] `protocol` crate defines the ultra-low latency binary UDP header (`AudioPacketHeader`) and pure audio packets (raw PCM float32 stereo).
- [x] `core` crate implements primary ports (`ProcessAudioUseCase`) and secondary ports (`AudioCapturePort`, `AudioStreamerPort`) guaranteeing pure sample pass-through.
- [x] `server` crate packages pure captured audio and transmits UDP datagrams over local network sockets.
- [x] `client` crate receives UDP datagrams and queues pure samples into a lock-free ring buffer for faithful playback.
- [x] Unit and in-memory integration tests validating transmission and reception of pure audio buffers with bit-exact data verification in loopback.

## Blocked by

None — can start immediately
