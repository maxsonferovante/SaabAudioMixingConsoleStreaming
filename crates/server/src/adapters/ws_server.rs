use audio_core::domain::VuMeterReading;
use audio_core::error::CoreError;
use audio_core::ports::secondary::TelemetryBroadcasterPort;
use futures_util::{SinkExt, StreamExt};
use protocol::{ControlCommandDto, TelemetryPacketDto};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

pub struct WebSocketTelemetryBroadcaster {
    tx: broadcast::Sender<String>,
}

impl WebSocketTelemetryBroadcaster {
    pub fn new() -> (Self, broadcast::Receiver<String>) {
        let (tx, rx) = broadcast::channel(64);
        (Self { tx }, rx)
    }

    pub fn sender(&self) -> broadcast::Sender<String> {
        self.tx.clone()
    }
}

impl TelemetryBroadcasterPort for WebSocketTelemetryBroadcaster {
    fn broadcast_vu(&self, reading: &VuMeterReading) -> Result<(), CoreError> {
        let dto = TelemetryPacketDto::VuMeter {
            peak_left: reading.peak_left,
            peak_right: reading.peak_right,
            rms_left: reading.rms_left,
            rms_right: reading.rms_right,
            is_clipping: reading.is_clipping,
        };

        if let Ok(json) = serde_json::to_string(&dto) {
            let _ = self.tx.send(json);
        }

        Ok(())
    }
}

pub struct WebSocketControlServer {
    bind_addr: SocketAddr,
    telemetry_tx: broadcast::Sender<String>,
}

impl WebSocketControlServer {
    pub fn new(bind_addr: SocketAddr, telemetry_tx: broadcast::Sender<String>) -> Self {
        Self { bind_addr, telemetry_tx }
    }

    pub async fn run<F>(&self, on_command: F) -> Result<(), std::io::Error>
    where
        F: Fn(ControlCommandDto) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("WebSocket Control Server listening on ws://{}", self.bind_addr);

        let on_command = Arc::new(on_command);

        while let Ok((stream, peer_addr)) = listener.accept().await {
            info!("Accepted WebSocket connection from {}", peer_addr);
            let telemetry_rx = self.telemetry_tx.subscribe();
            let on_command_clone = Arc::clone(&on_command);

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, telemetry_rx, on_command_clone).await {
                    warn!("WebSocket connection closed for {}: {:?}", peer_addr, e);
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection<F>(
    stream: TcpStream,
    mut telemetry_rx: broadcast::Receiver<String>,
    on_command: Arc<F>,
) -> Result<(), anyhow::Error>
where
    F: Fn(ControlCommandDto) + Send + Sync + 'static,
{
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    loop {
        tokio::select! {
            // Receive commands from client
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<ControlCommandDto>(&text) {
                            if let ControlCommandDto::Ping { client_timestamp_us } = cmd {
                                let server_timestamp_us = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_micros() as u64;
                                let pong = TelemetryPacketDto::Pong {
                                    client_timestamp_us,
                                    server_timestamp_us,
                                };
                                if let Ok(json) = serde_json::to_string(&pong) {
                                    let _ = ws_sender.send(Message::Text(json.into())).await;
                                }
                            } else {
                                on_command(cmd);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        error!("WS receive error: {:?}", e);
                        break;
                    }
                    _ => {}
                }
            }
            // Send telemetry to client
            telemetry = telemetry_rx.recv() => {
                if let Ok(json) = telemetry {
                    if let Err(e) = ws_sender.send(Message::Text(json.into())).await {
                        warn!("WS send error: {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
