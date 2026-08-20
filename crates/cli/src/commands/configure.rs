use anyhow::Result;
use cpal::traits::HostTrait;
use dialoguer::{Input, Select};
use std::process::Command;

use crate::config::{AdbConfig, AudioConfig, NetworkConfig, SaabConfig};

pub fn run_configure() -> Result<()> {
    println!("=== Saab Audio Mixing Console - Configuration Wizard ===\n");

    let mut current_config = SaabConfig::load().unwrap_or_default();

    // 1. Discover CoreAudio Devices
    let host = cpal::default_host();
    let mut detected_devices = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            if let Ok(name) = cpal::traits::DeviceTrait::name(&d) {
                detected_devices.push(name);
            }
        }
    }

    if detected_devices.is_empty() {
        detected_devices.push("BlackHole 16ch".to_string());
        detected_devices.push("BlackHole 2ch".to_string());
        detected_devices.push("Default Output Device".to_string());
    }

    // Determine default selection index
    let default_device_idx = detected_devices
        .iter()
        .position(|d| d.contains("BlackHole 16ch") || d == &current_config.audio.device_name)
        .unwrap_or(0);

    println!("[Audio Hardware Discovery]");
    let selected_device_idx = Select::new()
        .with_prompt("Select CoreAudio Input Driver for macOS Loopback")
        .items(&detected_devices)
        .default(default_device_idx)
        .interact()?;
    let selected_device = detected_devices[selected_device_idx].clone();

    // 2. Discover Android ADB Devices & IP
    println!("\n[Android Receiver Discovery]");
    let adb_output = Command::new("adb")
        .arg("devices")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut attached_devices = Vec::new();
    for line in adb_output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == "device" {
            attached_devices.push(parts[0].to_string());
        }
    }

    let mut auto_detected_ip = String::new();
    if !attached_devices.is_empty() {
        let ip_out = Command::new("adb")
            .args(["shell", "ip -f inet addr show wlan0 2>/dev/null | grep 'inet ' | awk '{print $2}' | cut -d/ -f1"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        if !ip_out.is_empty() && ip_out.contains('.') {
            auto_detected_ip = ip_out;
        }
    }

    let mode_options =
        vec!["Wi-Fi Network Streaming (Recommended)", "Direct USB Cable (ADB Port Forward)"];
    let mode_idx = Select::new()
        .with_prompt("Select Connection Mode")
        .items(&mode_options)
        .default(0)
        .interact()?;

    let (mode_str, default_ip) = if mode_idx == 0 {
        let default_ip = if !auto_detected_ip.is_empty() {
            auto_detected_ip
        } else if !current_config.network.android_ip.is_empty()
            && current_config.network.android_ip != "127.0.0.1"
        {
            current_config.network.android_ip
        } else {
            "192.168.15.5".to_string()
        };
        ("wifi", default_ip)
    } else {
        ("usb", "127.0.0.1".to_string())
    };

    let android_ip: String = Input::new()
        .with_prompt("Enter Android Device Target IP")
        .default(default_ip)
        .interact_text()?;

    let audio_port: u16 = Input::new()
        .with_prompt("Enter UDP Audio Streaming Port")
        .default(current_config.network.audio_port)
        .interact_text()?;

    let ws_port: u16 = Input::new()
        .with_prompt("Enter WebSocket Telemetry & Control Port")
        .default(current_config.network.ws_port)
        .interact_text()?;

    current_config.audio = AudioConfig { device_name: selected_device, sample_rate: 48000 };
    current_config.network =
        NetworkConfig { mode: mode_str.to_string(), android_ip, audio_port, ws_port };
    current_config.adb = AdbConfig {
        target_device: attached_devices.first().cloned().unwrap_or_else(|| "auto".to_string()),
        auto_reverse_port: true,
    };

    current_config.save()?;

    println!("\nConfiguration successfully saved to: {:?}", SaabConfig::config_file_path());
    println!("Run 'saab start' to launch background services.");

    Ok(())
}
