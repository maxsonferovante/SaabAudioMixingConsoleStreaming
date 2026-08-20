use audio_core::domain::{VuMeterReading, VuMeterState};
use iced::widget::{button, column, container, row, slider, text, vertical_space};
use iced::{
    alignment, executor, time, Alignment, Application, Border, Color, Command, Element, Length,
    Subscription, Theme,
};
use protocol::{ControlCommandDto, TelemetryPacketDto};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::info;

use crate::adapters::WebSocketClient;
use crate::ui::theme::*;

#[derive(Debug, Default, Clone)]
pub struct ConsoleFlags {
    pub server_ws_url: String,
}

pub struct ConsoleApp {
    server_ws_url: String,
    volume_db: f32,
    is_muted: bool,
    is_dimmed: bool,
    is_connected: bool,
    rtt_ms: f32,
    vu_meter_state: VuMeterState,
    current_vu: VuMeterReading,
    last_ping_time: Option<Instant>,
    ws_client: Option<WebSocketClient>,
    telemetry_rx: Option<ArcReceiver>,
}

pub type ArcReceiver =
    std::sync::Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<TelemetryPacketDto>>>;

#[derive(Debug, Clone)]
pub enum Message {
    VolumeChanged(f32),
    ToggleMute,
    ToggleDim,
    ResetVolume,
    Connected(WebSocketClient, ArcReceiver),
    ConnectionFailed,
    TelemetryReceived(TelemetryPacketDto),
    SendPing,
    PollTelemetry,
    Noop,
}

impl Application for ConsoleApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ConsoleFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let app = Self {
            server_ws_url: flags.server_ws_url.clone(),
            volume_db: 0.0,
            is_muted: false,
            is_dimmed: false,
            is_connected: false,
            rtt_ms: 0.0,
            vu_meter_state: VuMeterState::default(),
            current_vu: VuMeterReading::ZERO,
            last_ping_time: None,
            ws_client: None,
            telemetry_rx: None,
        };

        let ws_url = flags.server_ws_url;
        let connect_cmd = Command::perform(
            async move {
                let (tx, rx) = mpsc::unbounded_channel();
                match WebSocketClient::connect(&ws_url, move |telemetry| {
                    let _ = tx.send(telemetry);
                })
                .await
                {
                    Ok(client) => Ok((client, std::sync::Arc::new(tokio::sync::Mutex::new(rx)))),
                    Err(_) => Err(()),
                }
            },
            |result| match result {
                Ok((client, rx)) => Message::Connected(client, rx),
                Err(_) => Message::ConnectionFailed,
            },
        );

        (app, connect_cmd)
    }

    fn title(&self) -> String {
        format!(
            "Saab Audio Console - [{}] - {:.1} dB",
            if self.is_connected { "ONLINE" } else { "STANDBY" },
            self.volume_db
        )
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::VolumeChanged(val) => {
                self.volume_db = val;
                let client = self.ws_client.clone();
                let cmd = ControlCommandDto::SetMasterVolume { db: val };
                return Command::perform(
                    async move {
                        if let Some(c) = client {
                            let _ = c.send_command(cmd).await;
                        }
                    },
                    |_| Message::Noop,
                );
            }
            Message::ToggleMute => {
                self.is_muted = !self.is_muted;
                let client = self.ws_client.clone();
                let cmd = ControlCommandDto::SetMute { muted: self.is_muted };
                return Command::perform(
                    async move {
                        if let Some(c) = client {
                            let _ = c.send_command(cmd).await;
                        }
                    },
                    |_| Message::Noop,
                );
            }
            Message::ToggleDim => {
                self.is_dimmed = !self.is_dimmed;
                let client = self.ws_client.clone();
                let cmd = ControlCommandDto::SetDim { dimmed: self.is_dimmed };
                return Command::perform(
                    async move {
                        if let Some(c) = client {
                            let _ = c.send_command(cmd).await;
                        }
                    },
                    |_| Message::Noop,
                );
            }
            Message::ResetVolume => {
                return self.update(Message::VolumeChanged(0.0));
            }
            Message::Connected(client, rx) => {
                info!("UI: Connected to backend server");
                self.is_connected = true;
                self.ws_client = Some(client);
                self.telemetry_rx = Some(rx);
            }
            Message::ConnectionFailed => {
                self.is_connected = false;
                self.ws_client = None;
                self.telemetry_rx = None;
            }
            Message::TelemetryReceived(telemetry) => match telemetry {
                TelemetryPacketDto::VuMeter {
                    peak_left,
                    peak_right,
                    rms_left,
                    rms_right,
                    is_clipping,
                } => {
                    let raw =
                        VuMeterReading { peak_left, peak_right, rms_left, rms_right, is_clipping };
                    self.current_vu = self.vu_meter_state.process_reading(raw);
                }
                TelemetryPacketDto::Pong { .. } => {
                    if let Some(start) = self.last_ping_time.take() {
                        self.rtt_ms = start.elapsed().as_secs_f32() * 1000.0;
                    }
                }
                _ => {}
            },
            Message::SendPing => {
                if self.is_connected {
                    let client = self.ws_client.clone();
                    self.last_ping_time = Some(Instant::now());
                    let cmd = ControlCommandDto::Ping { client_timestamp_us: 0 };
                    return Command::perform(
                        async move {
                            if let Some(c) = client {
                                let _ = c.send_command(cmd).await;
                            }
                        },
                        |_| Message::Noop,
                    );
                } else {
                    let ws_url = self.server_ws_url.clone();
                    return Command::perform(
                        async move {
                            let (tx, rx) = mpsc::unbounded_channel();
                            match WebSocketClient::connect(&ws_url, move |telemetry| {
                                let _ = tx.send(telemetry);
                            })
                            .await
                            {
                                Ok(client) => {
                                    Ok((client, std::sync::Arc::new(tokio::sync::Mutex::new(rx))))
                                }
                                Err(_) => Err(()),
                            }
                        },
                        |result| match result {
                            Ok((client, rx)) => Message::Connected(client, rx),
                            Err(_) => Message::ConnectionFailed,
                        },
                    );
                }
            }
            Message::PollTelemetry => {
                if let Some(rx_arc) = self.telemetry_rx.clone() {
                    return Command::perform(
                        async move {
                            let mut guard = rx_arc.lock().await;
                            let mut messages = Vec::new();
                            while let Ok(msg) = guard.try_recv() {
                                messages.push(msg);
                            }
                            messages
                        },
                        |messages| {
                            if let Some(last) = messages.into_iter().last() {
                                Message::TelemetryReceived(last)
                            } else {
                                Message::Noop
                            }
                        },
                    );
                }
            }
            Message::Noop => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let tick = time::every(Duration::from_millis(16)).map(|_| Message::PollTelemetry);
        let ping = time::every(Duration::from_millis(1000)).map(|_| Message::SendPing);
        Subscription::batch(vec![tick, ping])
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // Status Bar Header
        let status_badge = if self.is_connected {
            text("[ONLINE]").size(13).style(iced::theme::Text::Color(METER_GREEN))
        } else {
            text("[OFFLINE]").size(13).style(iced::theme::Text::Color(METER_RED))
        };

        let latency_badge = text(format!("RTT: {:.1} ms", self.rtt_ms))
            .size(13)
            .style(iced::theme::Text::Color(TEXT_MUTED));

        let server_badge = text(format!("HOST: {}", self.server_ws_url))
            .size(13)
            .style(iced::theme::Text::Color(TEXT_MUTED));

        let header = row![status_badge, server_badge, latency_badge]
            .spacing(20)
            .align_items(Alignment::Center);

        // Volume Level Display
        let volume_text = if self.is_muted {
            text("MUTED").size(36).style(iced::theme::Text::Color(METER_RED))
        } else if self.is_dimmed {
            text(format!("{:.1} dB (DIM)", self.volume_db - 20.0))
                .size(36)
                .style(iced::theme::Text::Color(BUTTON_DIM_ACTIVE))
        } else {
            text(format!("{:.1} dB", self.volume_db))
                .size(36)
                .style(iced::theme::Text::Color(ACCENT_CYAN))
        };

        // Dual VU Meters Visualization
        let vu_left_bars = render_vu_bar(self.current_vu.peak_left);
        let vu_right_bars = render_vu_bar(self.current_vu.peak_right);

        let vu_meters = row![
            column![text("L").size(11).style(iced::theme::Text::Color(TEXT_MUTED)), vu_left_bars]
                .align_items(Alignment::Center)
                .spacing(4),
            column![text("R").size(11).style(iced::theme::Text::Color(TEXT_MUTED)), vu_right_bars]
                .align_items(Alignment::Center)
                .spacing(4),
        ]
        .spacing(16)
        .align_items(Alignment::Center);

        // Master Fader Slider (-60dB to +6dB)
        let fader = slider(-60.0_f32..=6.0_f32, self.volume_db, Message::VolumeChanged)
            .step(0.5_f32)
            .width(Length::Fixed(280.0));

        // Control Buttons (MUTE, DIM, RESET 0dB)
        let mute_btn = button(
            text(if self.is_muted { "MUTED" } else { "MUTE" })
                .size(15)
                .horizontal_alignment(alignment::Horizontal::Center),
        )
        .width(Length::Fixed(90.0))
        .padding(10)
        .on_press(Message::ToggleMute);

        let dim_btn = button(
            text(if self.is_dimmed { "DIMMED" } else { "DIM -20dB" })
                .size(15)
                .horizontal_alignment(alignment::Horizontal::Center),
        )
        .width(Length::Fixed(100.0))
        .padding(10)
        .on_press(Message::ToggleDim);

        let reset_btn =
            button(text("0 dB").size(15).horizontal_alignment(alignment::Horizontal::Center))
                .width(Length::Fixed(70.0))
                .padding(10)
                .on_press(Message::ResetVolume);

        let controls_row =
            row![mute_btn, dim_btn, reset_btn].spacing(12).align_items(Alignment::Center);

        let content = column![
            text("SAAB COCKPIT AUDIO CONSOLE").size(11).style(iced::theme::Text::Color(TEXT_MUTED)),
            vertical_space().height(Length::Fixed(8.0)),
            header,
            vertical_space().height(Length::Fixed(18.0)),
            volume_text,
            vertical_space().height(Length::Fixed(15.0)),
            vu_meters,
            vertical_space().height(Length::Fixed(20.0)),
            fader,
            vertical_space().height(Length::Fixed(25.0)),
            controls_row,
        ]
        .align_items(Alignment::Center)
        .spacing(8)
        .padding(24);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(iced::theme::Container::Box)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

struct SegmentStyle(Color);

impl iced::widget::container::StyleSheet for SegmentStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(self.0.into()),
            border: Border { radius: 1.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

fn render_vu_bar<'a>(peak: f32) -> Element<'a, Message> {
    let total_segments = 20;
    let lit_segments = ((peak * total_segments as f32).round() as usize).min(total_segments);

    let mut segments = Vec::new();
    for i in (0..total_segments).rev() {
        let is_lit = i < lit_segments;
        let color = if i >= 18 {
            if is_lit {
                METER_RED
            } else {
                Color::from_rgb(0.25, 0.05, 0.05)
            }
        } else if i >= 14 {
            if is_lit {
                METER_YELLOW
            } else {
                Color::from_rgb(0.25, 0.2, 0.02)
            }
        } else if is_lit {
            METER_GREEN
        } else {
            Color::from_rgb(0.05, 0.2, 0.1)
        };

        let block = container(vertical_space().height(Length::Fixed(4.0)))
            .width(Length::Fixed(28.0))
            .style(iced::theme::Container::Custom(Box::new(SegmentStyle(color))));

        segments.push(block.into());
    }

    column(segments).spacing(2).into()
}
