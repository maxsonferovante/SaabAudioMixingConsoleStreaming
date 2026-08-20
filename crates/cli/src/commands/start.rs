use anyhow::{Context, Result};
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tracing::info;

use crate::config::SaabConfig;
use crate::downloader::AndroidAssetDownloader;

pub fn run_start() -> Result<()> {
    let config = SaabConfig::load()?;
    let logs_dir = SaabConfig::logs_dir();
    let pids_dir = SaabConfig::pids_dir();

    fs::create_dir_all(&logs_dir)
        .with_context(|| format!("Failed to create logs directory at {:?}", logs_dir))?;
    fs::create_dir_all(&pids_dir)
        .with_context(|| format!("Failed to create pids directory at {:?}", pids_dir))?;

    println!("=== Starting Saab Audio Mixing Console Streaming Services ===\n");

    // 1. Terminate existing macOS server instances if running
    let server_pid_file = pids_dir.join("server.pid");
    if server_pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&server_pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                let _ = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .stderr(Stdio::null())
                    .status();
                thread::sleep(Duration::from_millis(300));
            }
        }
        let _ = fs::remove_file(&server_pid_file);
    }
    // Also cleanup any orphan server processes
    let _ =
        Command::new("pkill").args(["-f", "target/release/server"]).stderr(Stdio::null()).status();
    let _ =
        Command::new("pkill").args(["-f", "target/debug/server"]).stderr(Stdio::null()).status();

    // 2. Locate and spawn macOS Server Daemon
    let server_bin = find_server_binary()?;
    let server_log_file = logs_dir.join("server.log");
    let log_out = File::create(&server_log_file)
        .with_context(|| format!("Failed to open server log file at {:?}", server_log_file))?;
    let log_err = log_out.try_clone().context("Failed to clone log file handle")?;

    let ip_only =
        config.network.android_ip.split(':').next().unwrap_or(&config.network.android_ip).trim();
    let target_addr = format!("{}:{}", ip_only, config.network.audio_port);
    println!("[macOS Server] Launching background audio capture engine...");
    println!("  - Target Endpoint : {}", target_addr);
    println!("  - Audio Driver    : {}", config.audio.device_name);
    println!("  - Log Output      : {:?}", server_log_file);

    let child = Command::new(&server_bin)
        .arg(&target_addr)
        .arg(&config.audio.device_name)
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log_err))
        .spawn()
        .with_context(|| format!("Failed to spawn server binary {:?}", server_bin))?;

    let server_pid = child.id();
    fs::write(&server_pid_file, server_pid.to_string())
        .with_context(|| format!("Failed to write server PID to {:?}", server_pid_file))?;
    println!("  - Service PID     : {}\n", server_pid);

    // 3. Android Receiver Deployment via ADB
    println!("[Android Node] Checking device connectivity via ADB...");
    let adb_check = Command::new("adb").arg("devices").output();
    let has_adb_devices = match adb_check {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout);
            s.lines().skip(1).any(|l| l.contains("device"))
        }
        Err(_) => false,
    };

    if has_adb_devices {
        println!("[Android Node] Attached device detected. Ensuring client binary is present...");
        match AndroidAssetDownloader::get_client_binary_path() {
            Ok(bin_path) => {
                if let Err(e) = AndroidAssetDownloader::push_to_device(&bin_path, None) {
                    println!("[WARN] ADB push failed: {:?}. Continuing with streaming...", e);
                } else {
                    // Configure ADB ports
                    println!("[Android Node] Configuring ADB port forwarding...");
                    let _ =
                        Command::new("adb").args(["forward", "tcp:48480", "tcp:48480"]).status();
                    let _ = Command::new("adb").args(["reverse", "tcp:9001", "tcp:9001"]).status();

                    // Kill any previous instance on Android
                    let _ = Command::new("adb")
                        .args(["shell", "pkill -9 -f /data/local/tmp/client"])
                        .status();
                    thread::sleep(Duration::from_millis(200));

                    // Launch client in background on Android
                    println!("[Android Node] Spawning audio playback engine in background...");
                    let start_android = Command::new("adb")
                        .args([
                            "shell",
                            "nohup sh -c 'LD_LIBRARY_PATH=/data/local/tmp /data/local/tmp/client' > /data/local/tmp/client.log 2>&1 &",
                        ])
                        .status();
                    match start_android {
                        Ok(st) if st.success() => {
                            println!("  - Status: ACTIVE (Audio routed to 3.5mm P2 jack)");
                        }
                        _ => {
                            println!("[WARN] Android background start returned error code. Check 'saab logs --device-android'");
                        }
                    }
                }
            }
            Err(e) => {
                println!("[WARN] Could not obtain Android client binary: {:?}", e);
            }
        }
    } else {
        println!(
            "[INFO] No USB ADB device detected. Audio will stream over Wi-Fi directly to {}",
            target_addr
        );
    }

    println!("\n=== All Services Successfully Started ===");
    println!("Commands:");
    println!("  saab status       - Check live service status & telemetry");
    println!("  saab logs         - Stream real-time logs");
    println!("  saab studio       - Launch Studio Touch Console (Iced GUI)");
    println!("  saab stop         - Stop all active services\n");

    Ok(())
}

fn find_server_binary() -> Result<std::path::PathBuf> {
    // 1. Check release build in target
    let release_path = std::path::PathBuf::from("target/release/server");
    if release_path.exists() {
        return Ok(release_path);
    }

    // 2. Check debug build in target
    let debug_path = std::path::PathBuf::from("target/debug/server");
    if debug_path.exists() {
        return Ok(debug_path);
    }

    // 3. Fallback: check executable in current exe directory
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("server");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    // 4. Try building or running server via cargo
    info!("Compiling server binary...");
    let build_status = Command::new("cargo")
        .args(["build", "--bin", "server"])
        .status()
        .context("Failed to invoke cargo build for server binary")?;
    if !build_status.success() {
        anyhow::bail!("Failed to compile server binary via cargo");
    }

    Ok(std::path::PathBuf::from("target/debug/server"))
}
