use anyhow::Result;
use client::adapters::{CpalAudioPlayback, UdpAudioReceiver};
use client::ui::{ConsoleApp, ConsoleFlags};
use iced::{window, Application, Settings};
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting AudioMixingConsole Studio Client...");

    let udp_bind_addr: SocketAddr = "127.0.0.1:48480".parse()?;
    let ws_url = "ws://127.0.0.1:9001".to_string();

    // 48000 samples buffer = 1 second of stereo audio
    let ring_buffer = HeapRb::<f32>::new(48000 * 2);
    let (producer, consumer) = ring_buffer.split();

    // Start UDP Receiver
    let udp_receiver = UdpAudioReceiver::new();
    udp_receiver.start(udp_bind_addr, producer)?;

    // Start CPAL local playback (if audio output is available)
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
