use anyhow::Result;
use audio_core::application::MixerService;
use audio_core::domain::{AudioBuffer, DecibelVolume};
use audio_core::ports::primary::{AdjustVolumeUseCase, ProcessAudioUseCase, ToggleMuteUseCase};
use audio_core::ports::secondary::AudioCapturePort;
use protocol::ControlCommandDto;
use server::adapters::{
    MacAudioCapture, UdpAudioStreamer, WebSocketControlServer, WebSocketTelemetryBroadcaster,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting AudioMixingConsole Streaming Server (macOS)...");

    let target_ip = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("TARGET_UDP_ADDR").ok())
        .unwrap_or_else(|| "127.0.0.1:48480".to_string());

    let udp_bind_addr: SocketAddr = "0.0.0.0:0".parse()?;
    let udp_target_addr: SocketAddr = target_ip.parse()?;
    let ws_bind_addr: SocketAddr = "0.0.0.0:9001".parse()?;

    info!("Streaming UDP audio to target: {}", udp_target_addr);

    let streamer = Arc::new(UdpAudioStreamer::new(udp_bind_addr, udp_target_addr)?);
    let (telemetry_broadcaster, _rx) = WebSocketTelemetryBroadcaster::new();
    let telemetry_arc = Arc::new(telemetry_broadcaster);

    let mixer =
        Arc::new(Mutex::new(MixerService::new(Some(streamer), Some(telemetry_arc.clone()))));

    // Spawn WebSocket Control Server
    let ws_server = WebSocketControlServer::new(ws_bind_addr, telemetry_arc.sender());
    let mixer_for_ws = Arc::clone(&mixer);

    tokio::spawn(async move {
        let _ = ws_server
            .run(move |cmd| {
                if let Ok(mut m) = mixer_for_ws.lock() {
                    match cmd {
                        ControlCommandDto::SetMasterVolume { db } => {
                            m.set_master_volume(DecibelVolume::new(db));
                            info!("Volume set to {:.1} dB", db);
                        }
                        ControlCommandDto::SetMute { muted } => {
                            m.set_mute(muted);
                            info!("Mute set to {}", muted);
                        }
                        ControlCommandDto::SetDim { dimmed } => {
                            m.set_dim(dimmed);
                            info!("Dim set to {}", dimmed);
                        }
                        ControlCommandDto::Ping { .. } => {}
                    }
                }
            })
            .await;
    });

    // Start macOS Audio Capture
    let mut capture = MacAudioCapture::new(std::env::args().nth(2));
    let mixer_for_capture = Arc::clone(&mixer);

    let capture_result = capture.start_capture(Box::new(move |buffer: AudioBuffer| {
        if let Ok(mut m) = mixer_for_capture.lock() {
            let _ = m.process_block(buffer);
        }
    }));

    match capture_result {
        Ok(()) => {
            info!("macOS Audio Capture stream active. Real-time audio is being captured.");
        }
        Err(e) => {
            warn!("Hardware audio capture unavailable ({:?}). Running fallback generator...", e);
            let mixer_for_fallback = Arc::clone(&mixer);
            tokio::spawn(async move {
                let frame_size = 240;
                let mut interval = tokio::time::interval(Duration::from_millis(5));
                loop {
                    interval.tick().await;
                    let samples = vec![0.0; frame_size * 2];
                    if let Ok(buffer) = AudioBuffer::new(samples, 2, 48000) {
                        if let Ok(mut m) = mixer_for_fallback.lock() {
                            let _ = m.process_block(buffer);
                        }
                    }
                }
            });
        }
    }

    info!("Server is running. Press Ctrl+C to exit.");
    tokio::signal::ctrl_c().await?;
    info!("Shutting down AudioMixingConsole Server.");
    let _ = capture.stop_capture();

    Ok(())
}
