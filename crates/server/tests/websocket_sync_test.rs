use audio_core::application::MixerService;
use audio_core::domain::{DecibelVolume, MuteState};
use audio_core::ports::primary::{AdjustVolumeUseCase, ToggleMuteUseCase};
use client::adapters::WebSocketClient;
use protocol::{ControlCommandDto, TelemetryPacketDto};
use server::adapters::{WebSocketControlServer, WebSocketTelemetryBroadcaster};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_websocket_bidirectional_fader_and_mute_sync() {
    let ws_bind_addr: SocketAddr = "127.0.0.1:49200".parse().unwrap();
    let (telemetry_broadcaster, _rx) = WebSocketTelemetryBroadcaster::new();
    let telemetry_arc = Arc::new(telemetry_broadcaster);

    let mixer = Arc::new(Mutex::new(MixerService::new(None, Some(telemetry_arc.clone()))));
    let ws_server = WebSocketControlServer::new(ws_bind_addr, telemetry_arc.sender());

    let mixer_for_ws = Arc::clone(&mixer);
    tokio::spawn(async move {
        let _ = ws_server
            .run(move |cmd| {
                if let Ok(mut m) = mixer_for_ws.lock() {
                    match cmd {
                        ControlCommandDto::SetMasterVolume { db } => {
                            m.set_master_volume(DecibelVolume::new(db));
                        }
                        ControlCommandDto::SetMute { muted } => {
                            m.set_mute(muted);
                        }
                        ControlCommandDto::SetDim { dimmed } => {
                            m.set_dim(dimmed);
                        }
                        ControlCommandDto::Ping { .. } => {}
                    }
                }
            })
            .await;
    });

    // Wait for server listener to bind
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect Client
    let (telemetry_tx, mut telemetry_rx) = mpsc::channel::<TelemetryPacketDto>(16);
    let ws_url = "ws://127.0.0.1:49200";

    let client = WebSocketClient::connect(ws_url, move |telemetry| {
        let _ = telemetry_tx.try_send(telemetry);
    })
    .await
    .expect("connect client");

    // Test 1: Send SetMasterVolume (-12.0 dB)
    client
        .send_command(ControlCommandDto::SetMasterVolume { db: -12.0 })
        .await
        .expect("send volume");

    tokio::time::sleep(Duration::from_millis(50)).await;
    {
        let m = mixer.lock().unwrap();
        assert_eq!(m.get_master_volume().as_db(), -12.0);
    }

    // Test 2: Send SetMute (true)
    client.send_command(ControlCommandDto::SetMute { muted: true }).await.expect("send mute");

    tokio::time::sleep(Duration::from_millis(50)).await;
    {
        let m = mixer.lock().unwrap();
        assert_eq!(m.get_state(), MuteState::Muted);
    }

    // Test 3: Send SetDim (true) after unmute
    client.send_command(ControlCommandDto::SetMute { muted: false }).await.expect("send unmute");
    client.send_command(ControlCommandDto::SetDim { dimmed: true }).await.expect("send dim");

    tokio::time::sleep(Duration::from_millis(50)).await;
    {
        let m = mixer.lock().unwrap();
        assert_eq!(m.get_state(), MuteState::Dimmed);
    }

    // Test 4: Broadcast telemetry from server to client
    let reading = audio_core::domain::VuMeterReading {
        peak_left: 0.75,
        peak_right: 0.70,
        rms_left: 0.50,
        rms_right: 0.48,
        is_clipping: false,
    };
    audio_core::ports::secondary::TelemetryBroadcasterPort::broadcast_vu(&*telemetry_arc, &reading)
        .expect("broadcast vu");

    let received = tokio::time::timeout(Duration::from_millis(500), telemetry_rx.recv())
        .await
        .expect("timeout waiting for telemetry")
        .expect("received telemetry");

    match received {
        TelemetryPacketDto::VuMeter { peak_left, peak_right, .. } => {
            assert!((peak_left - 0.75).abs() < 1e-4);
            assert!((peak_right - 0.70).abs() < 1e-4);
        }
        other => panic!("Unexpected telemetry packet: {:?}", other),
    }
}
