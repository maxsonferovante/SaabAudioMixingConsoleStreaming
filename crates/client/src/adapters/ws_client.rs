use futures_util::{SinkExt, StreamExt};
use protocol::{ControlCommandDto, TelemetryPacketDto};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

pub struct WebSocketClient {
    command_tx: mpsc::Sender<ControlCommandDto>,
}

impl WebSocketClient {
    pub async fn connect<F>(url: &str, on_telemetry: F) -> Result<Self, anyhow::Error>
    where
        F: Fn(TelemetryPacketDto) + Send + Sync + 'static,
    {
        let (ws_stream, _) = connect_async(url).await?;
        info!("Connected to WebSocket server at {}", url);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (command_tx, mut command_rx) = mpsc::channel::<ControlCommandDto>(32);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(cmd) = command_rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&cmd) {
                            if let Err(e) = ws_sender.send(Message::Text(json.into())).await {
                                warn!("Failed to send command over WS: {:?}", e);
                                break;
                            }
                        }
                    }
                    Some(msg) = ws_receiver.next() => {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(telemetry) = serde_json::from_str::<TelemetryPacketDto>(&text) {
                                    on_telemetry(telemetry);
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Err(e) => {
                                error!("WS receiver error: {:?}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            info!("WebSocket client connection closed");
        });

        Ok(Self { command_tx })
    }

    pub async fn send_command(&self, command: ControlCommandDto) -> Result<(), anyhow::Error> {
        self.command_tx.send(command).await.map_err(|e| anyhow::anyhow!("Send error: {:?}", e))
    }
}
