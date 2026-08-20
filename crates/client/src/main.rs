use anyhow::Result;
use client::adapters::UdpAudioReceiver;
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[cfg(not(target_os = "android"))]
use client::adapters::CpalAudioPlayback;
#[cfg(not(target_os = "android"))]
use client::ui::{ConsoleApp, ConsoleFlags};
#[cfg(not(target_os = "android"))]
use iced::{window, Application, Settings};

#[cfg(target_os = "android")]
use client::adapters::OboeAudioPlayback;

#[cfg(not(target_os = "android"))]
fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting AudioMixingConsole Studio Client (Desktop)...");

    let udp_bind_addr: SocketAddr = "0.0.0.0:48480".parse()?;
    let ws_url = "ws://127.0.0.1:9001".to_string();

    let ring_buffer = HeapRb::<f32>::new(48000 * 2);
    let (producer, consumer) = ring_buffer.split();

    let udp_receiver = UdpAudioReceiver::new();
    udp_receiver.start(udp_bind_addr, producer)?;

    match CpalAudioPlayback::start(consumer) {
        Ok(_playback) => info!("Audio playback engine initialized on default device"),
        Err(e) => tracing::warn!("Hardware playback unavailable (simulation mode): {:?}", e),
    }

    let settings = Settings {
        flags: ConsoleFlags { server_ws_url: ws_url },
        window: window::Settings {
            size: iced::Size::new(420.0, 680.0),
            resizable: true,
            decorations: true,
            ..Default::default()
        },
        ..Default::default()
    };

    ConsoleApp::run(settings)?;

    Ok(())
}

#[cfg(target_os = "android")]
#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting AudioMixingConsole Android Client Engine (Oboe AAudio P2 output)...");

    let udp_bind_addr: SocketAddr = "0.0.0.0:48480".parse()?;

    // 48000 samples buffer = 1 second of stereo audio
    let ring_buffer = HeapRb::<f32>::new(48000 * 2);
    let (producer, consumer) = ring_buffer.split();

    // Start UDP Receiver listening on port 48480
    let udp_receiver = UdpAudioReceiver::new();
    udp_receiver.start(udp_bind_addr, producer)?;

    // Start Oboe AAudio hardware playback on 3.5mm P2 output
    let _playback = OboeAudioPlayback::start(consumer)?;
    info!("Oboe AAudio P2 output engine active. Receiving UDP audio...");

    // Keep running until interrupt signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Android audio engine.");

    Ok(())
}
