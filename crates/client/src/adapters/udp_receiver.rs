use byteorder::{ByteOrder, LittleEndian};
use protocol::{AudioPacketHeader, SampleFormat, HEADER_SIZE};
use ringbuf::traits::Producer;
use ringbuf::HeapProd;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

pub struct UdpAudioReceiver {
    running: Arc<AtomicBool>,
}

impl Default for UdpAudioReceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl UdpAudioReceiver {
    pub fn new() -> Self {
        Self { running: Arc::new(AtomicBool::new(false)) }
    }

    /// Starts the background UDP receiver thread, writing incoming samples directly to the lock-free producer
    pub fn start(
        &self,
        bind_addr: SocketAddr,
        mut producer: HeapProd<f32>,
    ) -> Result<(), std::io::Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;

        self.running.store(true, Ordering::SeqCst);
        let running_flag = Arc::clone(&self.running);

        info!("UDP Audio Receiver listening on {}", bind_addr);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];

            while running_flag.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((amt, _src)) => {
                        if amt < HEADER_SIZE {
                            continue;
                        }

                        match AudioPacketHeader::read_from_slice(&buf[..amt]) {
                            Ok((header, payload)) => {
                                if header.format != SampleFormat::F32Le {
                                    warn!("Unsupported sample format {:?}", header.format);
                                    continue;
                                }

                                let sample_count =
                                    (header.sample_count as usize) * (header.channels as usize);
                                let expected_bytes = sample_count * 4;

                                if payload.len() >= expected_bytes {
                                    for i in 0..sample_count {
                                        let sample =
                                            LittleEndian::read_f32(&payload[i * 4..(i + 1) * 4]);
                                        let _ = producer.try_push(sample);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid audio packet header: {:?}", e);
                            }
                        }
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        // Timeout allows checking running_flag
                        continue;
                    }
                    Err(e) => {
                        error!("UDP recv_from error: {:?}", e);
                        break;
                    }
                }
            }

            info!("UDP Audio Receiver thread terminated");
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
