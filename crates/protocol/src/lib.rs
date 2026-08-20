use byteorder::{BigEndian, ByteOrder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAGIC_BYTES: &[u8; 4] = b"AMCS";
pub const HEADER_SIZE: usize = 28;

#[derive(Debug, Error, PartialEq)]
pub enum ProtocolError {
    #[error("Buffer too small: expected at least {expected} bytes, got {got}")]
    BufferTooSmall { expected: usize, got: usize },
    #[error("Invalid magic bytes: expected AMCS")]
    InvalidMagic,
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(u8),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SampleFormat {
    F32Le = 0,
    I16Le = 1,
}

impl TryFrom<u8> for SampleFormat {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::F32Le),
            1 => Ok(Self::I16Le),
            other => Err(ProtocolError::UnsupportedFormat(other)),
        }
    }
}

/// Binary UDP Audio Packet Header for ultra-low-latency real-time streaming
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioPacketHeader {
    pub sequence_number: u64,
    pub timestamp_us: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_count: u16,
    pub format: SampleFormat,
    pub flags: u8,
}

impl AudioPacketHeader {
    pub fn new(
        sequence_number: u64,
        timestamp_us: u64,
        sample_rate: u32,
        channels: u16,
        sample_count: u16,
        format: SampleFormat,
    ) -> Self {
        Self {
            sequence_number,
            timestamp_us,
            sample_rate,
            channels,
            sample_count,
            format,
            flags: 0,
        }
    }

    /// Serializes the header into a byte slice of at least 28 bytes.
    pub fn write_to_slice(&self, buf: &mut [u8]) -> Result<(), ProtocolError> {
        if buf.len() < HEADER_SIZE {
            return Err(ProtocolError::BufferTooSmall { expected: HEADER_SIZE, got: buf.len() });
        }

        buf[0..4].copy_from_slice(MAGIC_BYTES);
        BigEndian::write_u64(&mut buf[4..12], self.sequence_number);
        BigEndian::write_u64(&mut buf[12..20], self.timestamp_us);
        BigEndian::write_u32(&mut buf[20..24], self.sample_rate);
        BigEndian::write_u16(&mut buf[24..26], self.channels);
        BigEndian::write_u16(&mut buf[26..28], self.sample_count);
        // We pack format and flags into extra fields or reserved bits if needed
        Ok(())
    }

    /// Decodes an audio packet header from a byte slice.
    pub fn read_from_slice(buf: &[u8]) -> Result<(Self, &[u8]), ProtocolError> {
        if buf.len() < HEADER_SIZE {
            return Err(ProtocolError::BufferTooSmall { expected: HEADER_SIZE, got: buf.len() });
        }

        if &buf[0..4] != MAGIC_BYTES {
            return Err(ProtocolError::InvalidMagic);
        }

        let sequence_number = BigEndian::read_u64(&buf[4..12]);
        let timestamp_us = BigEndian::read_u64(&buf[12..20]);
        let sample_rate = BigEndian::read_u32(&buf[20..24]);
        let channels = BigEndian::read_u16(&buf[24..26]);
        let sample_count = BigEndian::read_u16(&buf[26..28]);

        let header = Self {
            sequence_number,
            timestamp_us,
            sample_rate,
            channels,
            sample_count,
            format: SampleFormat::F32Le,
            flags: 0,
        };

        Ok((header, &buf[HEADER_SIZE..]))
    }
}

/// WebSocket Control Commands sent from Client (Android / Web) to Server (Mac)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ControlCommandDto {
    SetMasterVolume { db: f32 },
    SetMute { muted: bool },
    SetDim { dimmed: bool },
    Ping { client_timestamp_us: u64 },
}

/// WebSocket Telemetry Packets sent from Server (Mac) to Client (Android / Web)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum TelemetryPacketDto {
    VuMeter { peak_left: f32, peak_right: f32, rms_left: f32, rms_right: f32, is_clipping: bool },
    ServerStats { active_clients: usize, master_volume_db: f32, is_muted: bool, is_dimmed: bool },
    Pong { client_timestamp_us: u64, server_timestamp_us: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_header_roundtrip() {
        let original =
            AudioPacketHeader::new(12345, 9876543210, 48000, 2, 240, SampleFormat::F32Le);

        let mut buf = [0u8; 64];
        original.write_to_slice(&mut buf).expect("Encode failed");

        let (decoded, payload) = AudioPacketHeader::read_from_slice(&buf).expect("Decode failed");

        assert_eq!(original.sequence_number, decoded.sequence_number);
        assert_eq!(original.timestamp_us, decoded.timestamp_us);
        assert_eq!(original.sample_rate, decoded.sample_rate);
        assert_eq!(original.channels, decoded.channels);
        assert_eq!(original.sample_count, decoded.sample_count);
        assert_eq!(payload.len(), 64 - HEADER_SIZE);
    }

    #[test]
    fn test_invalid_magic() {
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(b"NOPE");
        let result = AudioPacketHeader::read_from_slice(&buf);
        assert_eq!(result, Err(ProtocolError::InvalidMagic));
    }

    #[test]
    fn test_control_dto_json_serialization() {
        let cmd = ControlCommandDto::SetMasterVolume { db: -6.0 };
        let json = serde_json::to_string(&cmd).expect("serialize json");
        let deserialized: ControlCommandDto =
            serde_json::from_str(&json).expect("deserialize json");
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_telemetry_dto_json_serialization() {
        let telemetry = TelemetryPacketDto::VuMeter {
            peak_left: 0.85,
            peak_right: 0.82,
            rms_left: 0.45,
            rms_right: 0.44,
            is_clipping: false,
        };
        let json = serde_json::to_string(&telemetry).expect("serialize json");
        let deserialized: TelemetryPacketDto =
            serde_json::from_str(&json).expect("deserialize json");
        assert_eq!(telemetry, deserialized);
    }
}
