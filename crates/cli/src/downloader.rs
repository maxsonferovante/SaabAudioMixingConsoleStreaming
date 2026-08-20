use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

use crate::config::SaabConfig;

pub struct AndroidAssetDownloader;

impl AndroidAssetDownloader {
    pub fn get_client_binary_path() -> Result<PathBuf> {
        // 1. Check local workspace build first (development mode)
        let local_workspace_binary = PathBuf::from("target/aarch64-linux-android/release/client");
        if local_workspace_binary.exists() {
            info!("Using local Android build artifact: {:?}", local_workspace_binary);
            return Ok(local_workspace_binary);
        }

        // 2. Check local cache
        let cache_dir = SaabConfig::cache_dir().join("bin");
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory at {:?}", cache_dir))?;
        let cached_binary = cache_dir.join("client");
        if cached_binary.exists() {
            info!("Using cached Android client binary: {:?}", cached_binary);
            return Ok(cached_binary);
        }

        // 3. Download from GitHub Release
        let version = env!("CARGO_PKG_VERSION");
        info!("Android client binary not found locally. Downloading v{} asset from GitHub Releases...", version);

        let url = format!(
            "https://github.com/maxsonferovante/SaabAudioMixingConsoleStreaming/releases/download/v{}/client-aarch64-linux-android",
            version
        );

        let client = reqwest::blocking::Client::builder()
            .user_agent("saab-cli")
            .build()
            .context("Failed to build HTTP client")?;

        let response = client.get(&url).send();
        match response {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes().context("Failed to read release binary bytes")?;
                fs::write(&cached_binary, bytes)
                    .with_context(|| format!("Failed to write binary to {:?}", cached_binary))?;
                info!("Successfully downloaded Android client binary to {:?}", cached_binary);
                Ok(cached_binary)
            }
            _ => {
                // Fallback: Check if target exists in parent workspace directory
                let fallback = PathBuf::from("../target/aarch64-linux-android/release/client");
                if fallback.exists() {
                    return Ok(fallback);
                }
                anyhow::bail!(
                    "Android client binary not found locally and could not be fetched from {}. Please build via ./scripts/build_android.sh or verify internet connectivity.",
                    url
                );
            }
        }
    }

    pub fn push_to_device(binary_path: &Path, serial: Option<&str>) -> Result<()> {
        info!("Deploying Android client binary to device (/data/local/tmp/client)...");

        let mut cmd = Command::new("adb");
        if let Some(s) = serial {
            if s != "auto" && !s.is_empty() {
                cmd.args(["-s", s]);
            }
        }
        cmd.args(["push", binary_path.to_str().unwrap_or_default(), "/data/local/tmp/client"]);

        let status = cmd.status().context("Failed to execute adb push")?;
        if !status.success() {
            anyhow::bail!("adb push failed with exit status: {:?}", status);
        }

        // Grant execute permissions
        let mut chmod_cmd = Command::new("adb");
        if let Some(s) = serial {
            if s != "auto" && !s.is_empty() {
                chmod_cmd.args(["-s", s]);
            }
        }
        chmod_cmd.args(["shell", "chmod", "+x", "/data/local/tmp/client"]);
        let chmod_status = chmod_cmd.status().context("Failed to execute adb chmod")?;
        if !chmod_status.success() {
            anyhow::bail!("adb chmod failed with exit status: {:?}", chmod_status);
        }

        info!("Android client binary successfully pushed and executable.");
        Ok(())
    }
}
