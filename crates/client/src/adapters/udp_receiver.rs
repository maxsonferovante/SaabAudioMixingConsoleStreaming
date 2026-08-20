use byteorder::{ByteOrder, LittleEndian};
use protocol::{AudioPacketHeader, SampleFormat, HEADER_SIZE};
use ringbuf::traits::Producer;
use ringbuf::HeapProd;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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

    /// Starts background UDP and TCP audio receivers feeding the lock-free ring buffer
    pub fn start(
        &self,
        bind_addr: SocketAddr,
        producer: HeapProd<f32>,
    ) -> Result<(), std::io::Error> {
        self.running.store(true, Ordering::SeqCst);
        let running_flag = Arc::clone(&self.running);
        let shared_producer = Arc::new(Mutex::new(producer));

        // 1. UDP Receiver Socket
        let udp_socket = UdpSocket::bind(bind_addr)?;
        udp_socket.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;
        info!("Audio Receiver (UDP) listening on {}", bind_addr);

        let running_udp = Arc::clone(&running_flag);
        let prod_udp = Arc::clone(&shared_producer);

        thread::spawn(move || {
            let mut buf = [0u8; 65536];
            let mut packet_counter: u64 = 0;

            while running_udp.load(Ordering::Relaxed) {
                match udp_socket.recv_from(&mut buf) {
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
                                    if let Ok(mut prod) = prod_udp.lock() {
                                        for i in 0..sample_count {
                                            let sample = LittleEndian::read_f32(
                                                &payload[i * 4..(i + 1) * 4],
                                            );
                                            let _ = prod.try_push(sample);
                                        }
                                    }

                                    packet_counter += 1;
                                    if packet_counter == 1 || packet_counter % 500 == 0 {
                                        info!(
                                            "UDP Receiver: processed packet #{} ({} samples, {}Hz) -> P2 output",
                                            header.sequence_number, header.sample_count, header.sample_rate
                                        );
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

        // 2. TCP Receiver Socket (for direct USB / ADB Reverse tethering)
        let tcp_listener = match TcpListener::bind(bind_addr) {
            Ok(l) => {
                let _ = l.set_nonblocking(true);
                info!("Audio Receiver (TCP / ADB Reverse) listening on {}", bind_addr);
                Some(l)
            }
            Err(e) => {
                warn!("TCP listener could not bind to {}: {:?}", bind_addr, e);
                None
            }
        };

        if let Some(listener) = tcp_listener {
            let running_tcp = Arc::clone(&running_flag);
            let prod_tcp = Arc::clone(&shared_producer);

            thread::spawn(move || {
                let mut header_buf = [0u8; HEADER_SIZE];
                let mut payload_buf = vec![0u8; 65536];
                let mut packet_counter: u64 = 0;

                while running_tcp.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, peer_addr)) => {
                            info!("TCP Audio Streamer connected from {}", peer_addr);
                            let _ = stream.set_nodelay(true);

                            while running_tcp.load(Ordering::Relaxed) {
                                if let Err(e) = stream.read_exact(&mut header_buf) {
                                    if e.kind() == std::io::ErrorKind::WouldBlock
                                        || e.kind() == std::io::ErrorKind::TimedOut
                                    {
                                        std::thread::sleep(std::time::Duration::from_millis(1));
                                        continue;
                                    }
                                    info!("TCP Audio Streamer disconnected: {:?}", e);
                                    break;
                                }

                                match AudioPacketHeader::read_from_slice(&header_buf) {
                                    Ok((header, _)) => {
                                        let sample_count = (header.sample_count as usize)
                                            * (header.channels as usize);
                                        let payload_bytes = sample_count * 4;

                                        if payload_buf.len() < payload_bytes {
                                            payload_buf.resize(payload_bytes, 0);
                                        }

                                        if let Err(e) =
                                            stream.read_exact(&mut payload_buf[..payload_bytes])
                                        {
                                            if e.kind() == std::io::ErrorKind::WouldBlock
                                                || e.kind() == std::io::ErrorKind::TimedOut
                                            {
                                                std::thread::sleep(std::time::Duration::from_millis(1));
                                                continue;
                                            }
                                            warn!("TCP payload read error: {:?}", e);
                                            break;
                                        }

                                        if let Ok(mut prod) = prod_tcp.lock() {
                                            for i in 0..sample_count {
                                                let sample = LittleEndian::read_f32(
                                                    &payload_buf[i * 4..(i + 1) * 4],
                                                );
                                                let _ = prod.try_push(sample);
                                            }
                                        }

                                        packet_counter += 1;
                                        if packet_counter == 1 || packet_counter % 500 == 0 {
                                            info!(
                                                "TCP/USB Receiver: processed packet #{} ({} samples, {}Hz) -> P2 output",
                                                header.sequence_number, header.sample_count, header.sample_rate
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Invalid TCP audio header: {:?}", e);
                                        break;
                                    }
                                }
                            }
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            thread::sleep(std::time::Duration::from_millis(50));
                        }
                        Err(e) => {
                            error!("TCP accept error: {:?}", e);
                            break;
                        }
                    }
                }

                info!("TCP Audio Receiver thread terminated");
            });
        }

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::traits::Split;
    use ringbuf::HeapRb;

    #[test]
    fn test_udp_receiver_start_stop() {
        let receiver = UdpAudioReceiver::new();
        let ring_buffer = HeapRb::<f32>::new(1024);
        let (prod, _cons) = ring_buffer.split();
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        assert!(receiver.start(bind_addr, prod).is_ok());
        receiver.stop();
    }
}
