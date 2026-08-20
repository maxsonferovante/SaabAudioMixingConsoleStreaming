use anyhow::Result;
use client::ui::{ConsoleApp, ConsoleFlags};
use iced::{window, Application, Settings};

use crate::config::SaabConfig;

pub fn run_studio() -> Result<()> {
    let config = SaabConfig::load().unwrap_or_default();
    let ws_url = format!("ws://127.0.0.1:{}", config.network.ws_port);

    println!("=== Launching Saab Studio Touch Console (Iced GUI) ===");
    println!("  - Target WebSocket: {}", ws_url);

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
