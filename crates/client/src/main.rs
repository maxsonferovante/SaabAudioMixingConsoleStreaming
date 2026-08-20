use anyhow::Result;
use client::adapters::{CpalAudioPlayback, UdpAudioReceiver, WebSocketClient};
use protocol::ControlCommandDto;
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting AudioMixingConsole Client Console...");

    let udp_bind_addr: SocketAddr = "127.0.0.1:48480".parse()?;
    let ws_url = "ws://127.0.0.1:9001";

    // 48000 samples buffer = 1 second of stereo audio
    let ring_buffer = HeapRb::<f32>::new(48000 * 2);
    let (producer, consumer) = ring_buffer.split();

    // Start UDP Receiver
    let udp_receiver = UdpAudioReceiver::new();
    udp_receiver.start(udp_bind_addr, producer)?;

    // Start CPAL local playback (if audio output is available)
    match CpalAudioPlayback::start(consumer) {
        Ok(_playback) => info!("Audio playback engine initialized on default device"),
        Err(e) => tracing::warn!("Could not start hardware playback (simulation mode): {:?}", e),
    }

    // Connect to WebSocket server
    info!("Connecting to WebSocket server at {}...", ws_url);
    match WebSocketClient::connect(ws_url, |telemetry| {
        info!("Telemetry received: {:?}", telemetry);
    })
    .await
    {
        Ok(ws_client) => {
            info!("WebSocket connected. Sending initial volume calibration...");
            let _ = ws_client.send_command(ControlCommandDto::SetMasterVolume { db: 0.0 }).await;
        }
        Err(e) => tracing::warn!("WebSocket server not yet available: {:?}", e),
    }

    info!("Client console running. Waiting for audio packets...");
    tokio::signal::ctrl_c().await?;
    info!("Exiting AudioMixingConsole Client.");

    Ok(())
}
