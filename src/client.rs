//! BiWi UDP Client
//! Fast UDP-based client with automatic packet loss recovery

use crate::message::BiWiMessage;
use crate::network::{PacketManager, PacketType, UdpPacket};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// BiWi UDP Client
pub struct BiWiUdpClient {
    socket: Arc<UdpSocket>,
    server_addr: SocketAddr,
    packet_manager: Arc<Mutex<PacketManager>>,
    message_tx: Sender<Vec<u8>>,
    message_rx: Receiver<Vec<u8>>,
    running: Arc<Mutex<bool>>,
}

impl BiWiUdpClient {
    /// Create and connect a new UDP client
    pub fn connect(server_addr: &str) -> io::Result<Self> {
        let server_addr: SocketAddr = server_addr.parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid address"))?;

        // Bind to any local address
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(1)))?;

        println!(
            "[BiWi UDP] Client connected to {}",
            server_addr
        );

        let (tx, rx) = channel();

        let client = BiWiUdpClient {
            socket: Arc::new(socket),
            server_addr,
            packet_manager: Arc::new(Mutex::new(PacketManager::new())),
            message_tx: tx,
            message_rx: rx,
            running: Arc::new(Mutex::new(true)),
        };

        // Start receive loop
        let socket = Arc::clone(&client.socket);
        let packet_manager = Arc::clone(&client.packet_manager);
        let tx = client.message_tx.clone();
        let running = Arc::clone(&client.running);
        let server_addr = client.server_addr;

        thread::spawn(move || {
            let mut buf = vec![0u8; 65536];

            while *running.lock().unwrap() {
                match socket.recv_from(&mut buf) {
                    Ok((n, addr)) if addr == server_addr => {
                        let packet_data = &buf[..n];

                        if let Ok(packet) = UdpPacket::from_bytes(packet_data) {
                            let mut pm = packet_manager.lock().unwrap();

                            match packet.packet_type {
                                PacketType::Data => {
                                    // Record received and send ACK
                                    if pm.record_received(packet.sequence) {
                                        let ack = pm.create_ack_packet(packet.sequence);
                                        let _ = socket.send_to(&ack.to_bytes(), server_addr);

                                        // Emit message
                                        let _ = tx.send(packet.payload);
                                    }
                                }
                                PacketType::Ack => {
                                    pm.handle_ack(packet.ack_number);
                                }
                                PacketType::Pong => {
                                    // Keep-alive response received
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(_) => {
                        // Packet from wrong source, ignore
                    }
                    Err(_) => {
                        // Timeout - check for retransmits needed
                        let mut pm = packet_manager.lock().unwrap();
                        let retransmits = pm.get_retransmit_packets();
                        for (packet, _) in retransmits {
                            let _ = socket.send_to(&packet.to_bytes(), server_addr);
                        }
                    }
                }
            }
        });

        Ok(client)
    }

    /// Send a message to the server
    pub fn send(&self, message: &BiWiMessage) -> io::Result<()> {
        let msg_bytes = message.to_vec();
        let mut pm = self.packet_manager.lock().unwrap();
        let packets = pm.create_packets(&msg_bytes);

        for packet in packets {
            self.socket.send_to(&packet.to_bytes(), self.server_addr)?;
        }

        Ok(())
    }

    /// Try to receive a message (non-blocking)
    pub fn try_recv(&self) -> Option<BiWiMessage> {
        self.message_rx.try_recv().ok().and_then(|data| {
            BiWiMessage::from_buffer(&data).ok()
        })
    }

    /// Receive a message (blocking)
    pub fn recv(&self) -> io::Result<BiWiMessage> {
        self.message_rx
            .recv()
            .ok()
            .and_then(|data| BiWiMessage::from_buffer(&data).ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionReset, "Channel closed"))
    }

    /// Receive with timeout
    pub fn recv_timeout(&self, timeout: Duration) -> io::Result<BiWiMessage> {
        match self.message_rx.recv_timeout(timeout) {
            Ok(data) => BiWiMessage::from_buffer(&data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "Recv timeout")),
        }
    }

    /// Check if connected and receiving
    pub fn is_active(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Disconnect from server
    pub fn disconnect(&mut self) {
        *self.running.lock().unwrap() = false;
    }

    /// Send a ping (keep-alive)
    pub fn ping(&self) -> io::Result<()> {
        let mut pm = self.packet_manager.lock().unwrap();
        let ping = pm.create_ping_packet();
        self.socket.send_to(&ping.to_bytes(), self.server_addr)?;
        Ok(())
    }
}

impl Drop for BiWiUdpClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
