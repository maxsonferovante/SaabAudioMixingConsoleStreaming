use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod commands;
mod config;
mod downloader;

#[derive(Parser, Debug)]
#[command(
    name = "saab",
    about = "Ultra-low-latency distributed audio streaming & mixing console CLI for macOS and Android",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    /// Interactive configuration wizard for CoreAudio drivers, network IP, and ADB settings
    Configure,
    /// Start macOS audio capture server and Android receiver in background services
    Start,
    /// Stop all active audio streaming services cleanly
    Stop,
    /// Display real-time service status, PIDs, active audio driver, and latency
    Status,
    /// Stream live logs from macOS server or Android receiver
    Logs {
        /// Stream macOS audio server logs
        #[arg(long)]
        server_mac: bool,
        /// Stream Android receiver logs via ADB
        #[arg(long)]
        device_android: bool,
    },
    /// Launch the Dedicated Studio Touch Console (Iced GUI)
    Studio,
}

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let cli = Cli::parse();

    match cli.command {
        Commands::Configure => commands::run_configure()?,
        Commands::Start => commands::run_start()?,
        Commands::Stop => commands::run_stop()?,
        Commands::Status => commands::run_status()?,
        Commands::Logs { server_mac, device_android } => {
            commands::run_logs(server_mac, device_android)?
        }
        Commands::Studio => commands::run_studio()?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_configure() {
        let cli = Cli::try_parse_from(["saab", "configure"]).unwrap();
        assert_eq!(cli.command, Commands::Configure);
    }

    #[test]
    fn test_cli_parsing_start_stop_status() {
        assert_eq!(Cli::try_parse_from(["saab", "start"]).unwrap().command, Commands::Start);
        assert_eq!(Cli::try_parse_from(["saab", "stop"]).unwrap().command, Commands::Stop);
        assert_eq!(Cli::try_parse_from(["saab", "status"]).unwrap().command, Commands::Status);
        assert_eq!(Cli::try_parse_from(["saab", "studio"]).unwrap().command, Commands::Studio);
    }

    #[test]
    fn test_cli_parsing_logs_flags() {
        let logs_mac = Cli::try_parse_from(["saab", "logs", "--server-mac"]).unwrap();
        assert_eq!(logs_mac.command, Commands::Logs { server_mac: true, device_android: false });

        let logs_android = Cli::try_parse_from(["saab", "logs", "--device-android"]).unwrap();
        assert_eq!(
            logs_android.command,
            Commands::Logs { server_mac: false, device_android: true }
        );
    }
}
