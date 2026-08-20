use anyhow::Result;
use std::process::Command;

use crate::config::SaabConfig;

pub fn run_logs(server_mac: bool, device_android: bool) -> Result<()> {
    if device_android {
        println!("=== Streaming Android Receiver Logs (ADB) ===\n(Press Ctrl+C to exit)\n");
        let _ = Command::new("adb")
            .args(["shell", "tail -n 50 -f /data/local/tmp/client.log 2>/dev/null"])
            .status();
        return Ok(());
    }

    if server_mac || !device_android {
        let log_file = SaabConfig::logs_dir().join("server.log");
        if !log_file.exists() {
            println!("No server log file found at {:?}. Has 'saab start' been run?", log_file);
            return Ok(());
        }
        println!(
            "=== Streaming macOS Server Logs ===\nFile: {:?}\n(Press Ctrl+C to exit)\n",
            log_file
        );
        let _ = Command::new("tail")
            .args(["-n", "50", "-f", log_file.to_str().unwrap_or_default()])
            .status();
    }

    Ok(())
}
