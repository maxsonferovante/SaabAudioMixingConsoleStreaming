use anyhow::Result;
use audio_core::application::MixerService;
use audio_core::domain::{AudioBuffer, DecibelVolume};
use audio_core::ports::primary::{AdjustVolumeUseCase, ProcessAudioUseCase, ToggleMuteUseCase};
use protocol::ControlCommandDto;
use server::adapters::{UdpAudioStreamer, WebSocketControlServer, WebSocketTelemetryBroadcaster};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting AudioMixingConsole Streaming Server (macOS)...");

    let udp_bind_addr: SocketAddr = "0.0.0.0:0".parse()?;
    let udp_target_addr: SocketAddr = "127.0.0.1:48480".parse()?;
    let ws_bind_addr: SocketAddr = "0.0.0.0:9001".parse()?;

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

    info!("Server audio loop active. Streaming 48kHz stereo frames to UDP {}", udp_target_addr);

    // Audio capture & streaming loop (240 samples = 5ms at 48kHz)
    let frame_size = 240;
    let mut interval = tokio::time::interval(Duration::from_millis(5));

    loop {
        interval.tick().await;

        // In ticket 01, we generate pure sine/raw pass-through audio frames to test loopback
        let mut samples = Vec::with_capacity(frame_size * 2);
        for _ in 0..frame_size {
            samples.push(0.0);
            samples.push(0.0);
        }

        if let Ok(buffer) = AudioBuffer::new(samples, 2, 48000) {
            if let Ok(mut m) = mixer.lock() {
                let _ = m.process_block(buffer);
            }
        }
    }
}
