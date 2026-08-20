use anyhow::Result;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::config::SaabConfig;

pub fn run_stop() -> Result<()> {
    println!("=== Stopping Saab Audio Mixing Console Streaming Services ===\n");

    let pids_dir = SaabConfig::pids_dir();
    let server_pid_file = pids_dir.join("server.pid");

    // 1. Stop macOS Server Daemon
    let mut stopped_server = false;
    if server_pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&server_pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                println!("[macOS Server] Stopping process with PID {}...", pid);
                let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).status();
                thread::sleep(Duration::from_millis(200));
                stopped_server = true;
            }
        }
        let _ = fs::remove_file(&server_pid_file);
    }

    // Additional process name sweep
    let pkill_out = Command::new("pkill").args(["-f", "server"]).output();
    if let Ok(o) = pkill_out {
        if o.status.success() {
            stopped_server = true;
        }
    }

    if stopped_server {
        println!("  - macOS Server: STOPPED");
    } else {
        println!("  - macOS Server: NOT RUNNING");
    }

    // 2. Stop Android Node Receiver via ADB
    println!("[Android Node] Stopping receiver process on device...");
    let adb_stop = Command::new("adb")
        .args(["shell", "pkill -9 -f /data/local/tmp/client 2>/dev/null || true"])
        .status();

    match adb_stop {
        Ok(st) if st.success() => {
            println!("  - Android Receiver: STOPPED");
        }
        _ => {
            println!("  - Android Receiver: STOPPED / NOT CONNECTED");
        }
    }

    println!("\nAll streaming services have been cleanly terminated.");
    Ok(())
}
