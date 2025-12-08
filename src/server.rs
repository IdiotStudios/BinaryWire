//! BiWi UDP Server
//! Fast UDP-based server with automatic packet loss recovery

use crate::message::BiWiMessage;
use crate::network::{PacketManager, PacketType, UdpPacket};
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type ConnectionId = String;

/// Represents a connected client
pub struct ClientConnection {
    pub id: ConnectionId,
    pub addr: SocketAddr,
    pub packet_manager: PacketManager,
    pub last_activity: std::time::Instant,
}

/// BiWi UDP Server - Simple synchronous implementation
pub struct BiWiUdpServer {
    pub socket: UdpSocket,
    pub port: u16,
    pub host: String,
    pub connections: Arc<Mutex<HashMap<ConnectionId, ClientConnection>>>,
}

impl BiWiUdpServer {
    /// Create a new UDP server
    pub fn new(host: &str, port: u16) -> io::Result<Self> {
        let addr = format!("{}:{}", host, port);
        let socket = UdpSocket::bind(&addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        println!("[BiWi UDP] Server listening on {}", addr);

        Ok(BiWiUdpServer {
            socket,
            port,
            host: host.to_string(),
            connections: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Receive next packet and return (client_id, message) if complete
    pub fn recv_packet(&mut self) -> Option<(ConnectionId, BiWiMessage)> {
        let mut buf = vec![0u8; 65536];

        match self.socket.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let packet_data = &buf[..n];

                if let Ok(packet) = UdpPacket::from_bytes(packet_data) {
                    let client_id = addr.to_string();
                    let mut conns = self.connections.lock().unwrap();

                    // Get or create connection
                    let conn = conns
                        .entry(client_id.clone())
                        .or_insert_with(|| ClientConnection {
                            id: client_id.clone(),
                            addr,
                            packet_manager: PacketManager::new(),
                            last_activity: std::time::Instant::now(),
                        });

                    conn.last_activity = std::time::Instant::now();

                    // Handle different packet types
                    match packet.packet_type {
                        PacketType::Data => {
                            // Send ACK back
                            let ack_packet = conn.packet_manager.create_ack_packet(packet.sequence);
                            let _ = self.socket.send_to(&ack_packet.to_bytes(), addr);

                            // Check for duplicates
                            if conn.packet_manager.record_received(packet.sequence) {
                                // New packet - try to decode
                                match BiWiMessage::from_buffer(&packet.payload) {
                                    Ok(msg) => return Some((client_id, msg)),
                                    Err(_) => {} // Incomplete message, wait for more
                                }
                            }
                        }
                        PacketType::Ack => {
                            conn.packet_manager.handle_ack(packet.ack_number);
                        }
                        PacketType::Ping => {
                            let pong = UdpPacket {
                                packet_type: PacketType::Pong,
                                sequence: 0,
                                ack_number: packet.sequence,
                                flags: 0,
                                payload: Vec::new(),
                            };
                            let _ = self.socket.send_to(&pong.to_bytes(), addr);
                        }
                        _ => {}
                    }
                }
                None
            }
            Err(_) => {
                // Timeout - check for retransmits
                let mut conns = self.connections.lock().unwrap();
                for conn in conns.values_mut() {
                    let retransmits = conn.packet_manager.get_retransmit_packets();
                    for (packet, _) in retransmits {
                        let _ = self.socket.send_to(&packet.to_bytes(), conn.addr);
                    }
                }

                // Clean up stale connections
                let timeout = Duration::from_secs(30);
                conns.retain(|_, conn| conn.last_activity.elapsed() < timeout);

                None
            }
        }
    }

    /// Send a message to a specific client
    pub fn send_to(&self, client_id: &str, message: &BiWiMessage) -> io::Result<()> {
        let msg_bytes = message.to_vec();
        let mut conns = self.connections.lock().unwrap();

        if let Some(conn) = conns.get_mut(client_id) {
            let packets = conn.packet_manager.create_packets(&msg_bytes);
            for packet in packets {
                self.socket.send_to(&packet.to_bytes(), conn.addr)?;
            }
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Client not found",
            ))
        }
    }

    /// Broadcast a message to all connected clients
    pub fn broadcast(&self, message: &BiWiMessage) -> io::Result<()> {
        let msg_bytes = message.to_vec();
        let conns = self.connections.lock().unwrap();

        for conn in conns.values() {
            let mut pm = PacketManager::new();
            let packets = pm.create_packets(&msg_bytes);
            for packet in packets {
                self.socket.send_to(&packet.to_bytes(), conn.addr)?;
            }
        }
        Ok(())
    }

    /// Get all connected clients
    pub fn get_connections(&self) -> Vec<(ConnectionId, SocketAddr)> {
        self.connections
            .lock()
            .unwrap()
            .iter()
            .map(|(id, conn)| (id.clone(), conn.addr))
            .collect()
    }
}
