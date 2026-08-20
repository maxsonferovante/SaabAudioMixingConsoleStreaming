use audio_core::domain::AudioBuffer;
use audio_core::ports::secondary::AudioStreamerPort;
use client::adapters::UdpAudioReceiver;
use ringbuf::traits::{Consumer, Split};
use ringbuf::HeapRb;
use server::adapters::UdpAudioStreamer;
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::test]
async fn test_end_to_end_udp_audio_streaming() {
    let receiver_addr: SocketAddr = "127.0.0.1:49152".parse().unwrap();
    let sender_bind: SocketAddr = "127.0.0.1:0".parse().unwrap();

    let ring_buffer = HeapRb::<f32>::new(4096);
    let (producer, mut consumer) = ring_buffer.split();

    // Start Receiver
    let receiver = UdpAudioReceiver::new();
    receiver.start(receiver_addr, producer).expect("start receiver");

    // Wait a brief moment for socket bind
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Start Streamer
    let streamer = UdpAudioStreamer::new(sender_bind, receiver_addr).expect("create streamer");

    // Send known pure test signal (4 distinct floating-point samples)
    let test_samples = vec![0.1234, -0.5678, 0.9876, -0.4321];
    let buffer = AudioBuffer::new(test_samples.clone(), 2, 48000).expect("valid buffer");

    streamer.stream_audio(&buffer, 1, 1000).expect("stream audio packet");

    // Wait for packet arrival
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify samples received in ring buffer
    let mut received_samples = Vec::new();
    while let Some(sample) = consumer.try_pop() {
        received_samples.push(sample);
    }

    receiver.stop();

    assert_eq!(received_samples.len(), test_samples.len());
    for (i, (&expected, &actual)) in test_samples.iter().zip(received_samples.iter()).enumerate() {
        assert!(
            (expected - actual).abs() < 1e-6,
            "Sample {} mismatch: expected {}, got {}",
            i,
            expected,
            actual
        );
    }
}
