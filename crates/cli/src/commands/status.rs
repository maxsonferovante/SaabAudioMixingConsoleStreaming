use anyhow::Result;
use std::fs;
use std::process::Command;

use crate::config::SaabConfig;

pub fn run_status() -> Result<()> {
    let config = SaabConfig::load().unwrap_or_default();
    let pids_dir = SaabConfig::pids_dir();
    let server_pid_file = pids_dir.join("server.pid");

    println!("=== Saab Audio Mixing Console - Service Status ===\n");

    // 1. macOS Server Status
    println!("[macOS Audio Server]");
    let mut server_running = false;
    let mut server_pid_display = "NONE".to_string();

    if server_pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&server_pid_file) {
            let pid = pid_str.trim();
            // Check if PID is alive via kill -0
            let check = Command::new("kill").args(["-0", pid]).status();
            if let Ok(st) = check {
                if st.success() {
                    server_running = true;
                    server_pid_display = pid.to_string();
                }
            }
        }
    }

    if server_running {
        println!("  - Status          : RUNNING (PID: {})", server_pid_display);
    } else {
        println!("  - Status          : STOPPED");
    }
    println!("  - Audio Driver    : {}", config.audio.device_name);
    println!("  - Target Endpoint : {}:{}", config.network.android_ip, config.network.audio_port);
    println!("  - WebSocket Host  : ws://127.0.0.1:{}", config.network.ws_port);

    // 2. Android Node Status
    println!("\n[Android Audio Node]");
    let adb_out = Command::new("adb").arg("devices").output();
    let mut attached_device = "NOT DETECTED".to_string();
    if let Ok(o) = adb_out {
        let s = String::from_utf8_lossy(&o.stdout);
        for line in s.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                attached_device = parts[0].to_string();
                break;
            }
        }
    }
    println!("  - ADB Device      : {}", attached_device);

    let mut android_running = false;
    if attached_device != "NOT DETECTED" {
        let pgrep_out = Command::new("adb")
            .args(["shell", "pgrep -f /data/local/tmp/client 2>/dev/null"])
            .output();
        if let Ok(o) = pgrep_out {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !s.is_empty() {
                android_running = true;
                println!("  - Node Status     : RUNNING (PID: {}) -> P2 3.5mm Jack", s);
            }
        }
    }

    if !android_running {
        println!("  - Node Status     : STOPPED");
    }

    println!("\nConfig File: {:?}", SaabConfig::config_file_path());
    println!("Logs Dir   : {:?}", SaabConfig::logs_dir());
    Ok(())
}
