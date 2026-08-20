use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaabConfig {
    pub version: String,
    pub audio: AudioConfig,
    pub network: NetworkConfig,
    pub adb: AdbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioConfig {
    pub device_name: String,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkConfig {
    pub mode: String,
    pub android_ip: String,
    pub audio_port: u16,
    pub ws_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdbConfig {
    pub target_device: String,
    pub auto_reverse_port: bool,
}

impl Default for SaabConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            audio: AudioConfig {
                device_name: "BlackHole 16ch".to_string(),
                sample_rate: 48000,
            },
            network: NetworkConfig {
                mode: "wifi".to_string(),
                android_ip: "127.0.0.1".to_string(),
                audio_port: 48480,
                ws_port: 9001,
            },
            adb: AdbConfig {
                target_device: "auto".to_string(),
                auto_reverse_port: true,
            },
        }
    }
}

impl SaabConfig {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("saab")
    }

    pub fn config_file_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn logs_dir() -> PathBuf {
        Self::config_dir().join("logs")
    }

    pub fn pids_dir() -> PathBuf {
        Self::config_dir().join("pids")
    }

    pub fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("~/.cache"))
            .join("saab")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_file_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;
        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON structure in config file {:?}", path))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_file_path();
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize configuration to JSON")?;
        fs::write(path, json)
            .with_context(|| format!("Failed to write configuration file at {:?}", path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = SaabConfig::default();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.audio.device_name, "BlackHole 16ch");
        assert_eq!(config.audio.sample_rate, 48000);
        assert_eq!(config.network.audio_port, 48480);
        assert_eq!(config.network.ws_port, 9001);
        assert!(config.adb.auto_reverse_port);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original = SaabConfig {
            version: "1.0".to_string(),
            audio: AudioConfig {
                device_name: "BlackHole 2ch".to_string(),
                sample_rate: 96000,
            },
            network: NetworkConfig {
                mode: "wifi".to_string(),
                android_ip: "192.168.15.5".to_string(),
                audio_port: 48480,
                ws_port: 9001,
            },
            adb: AdbConfig {
                target_device: "device-123".to_string(),
                auto_reverse_port: true,
            },
        };

        let json = serde_json::to_string_pretty(&original).unwrap();
        let deserialized: SaabConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }
}
