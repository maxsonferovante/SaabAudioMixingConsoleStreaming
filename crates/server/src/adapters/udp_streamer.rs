use audio_core::domain::AudioBuffer;
use audio_core::error::CoreError;
use audio_core::ports::secondary::AudioStreamerPort;
use byteorder::{ByteOrder, LittleEndian};
use protocol::{AudioPacketHeader, SampleFormat, HEADER_SIZE};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Mutex;

pub struct UdpAudioStreamer {
    socket: UdpSocket,
    target_addr: Mutex<SocketAddr>,
    buffer: Mutex<Vec<u8>>,
}

impl UdpAudioStreamer {
    pub fn new(bind_addr: SocketAddr, target_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        // Set non-blocking for real-time safety
        socket.set_nonblocking(true)?;

        Ok(Self {
            socket,
            target_addr: Mutex::new(target_addr),
            buffer: Mutex::new(vec![0u8; 4096]),
        })
    }

    pub fn set_target_addr(&self, addr: SocketAddr) {
        if let Ok(mut guard) = self.target_addr.lock() {
            *guard = addr;
        }
    }
}

impl AudioStreamerPort for UdpAudioStreamer {
    fn stream_audio(
        &self,
        buffer: &AudioBuffer,
        sequence_number: u64,
        timestamp_us: u64,
    ) -> Result<(), CoreError> {
        let samples = buffer.samples();
        let payload_byte_len = samples.len() * 4; // float32 = 4 bytes
        let total_len = HEADER_SIZE + payload_byte_len;

        let mut buf_guard =
            self.buffer.lock().map_err(|_| CoreError::StreamingError("Mutex poison".into()))?;
        if buf_guard.len() < total_len {
            buf_guard.resize(total_len, 0);
        }

        let header = AudioPacketHeader::new(
            sequence_number,
            timestamp_us,
            buffer.sample_rate(),
            buffer.channels(),
            buffer.frame_count() as u16,
            SampleFormat::F32Le,
        );

        header
            .write_to_slice(&mut buf_guard[..HEADER_SIZE])
            .map_err(|e| CoreError::StreamingError(format!("Header write failed: {:?}", e)))?;

        // Write raw uncompressed float32 samples directly into payload slice
        let payload_slice = &mut buf_guard[HEADER_SIZE..total_len];
        for (i, &sample) in samples.iter().enumerate() {
            LittleEndian::write_f32(&mut payload_slice[i * 4..(i + 1) * 4], sample);
        }

        let target = *self
            .target_addr
            .lock()
            .map_err(|_| CoreError::StreamingError("Mutex poison".into()))?;
        let _ = self.socket.send_to(&buf_guard[..total_len], target);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_streamer_packet_generation() {
        let sender = UdpAudioStreamer::new(
            "127.0.0.1:0".parse().unwrap(),
            "127.0.0.1:9999".parse().unwrap(),
        )
        .expect("create sender");

        let test_buffer =
            AudioBuffer::new(vec![0.25, -0.5, 0.75, -1.0], 2, 48000).expect("valid buffer");
        let result = sender.stream_audio(&test_buffer, 1, 1000);
        assert!(result.is_ok());
    }
}
