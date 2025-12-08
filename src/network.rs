//! BiWi UDP Network Module
//! Provides fast UDP-based transport with packet loss handling
//! Features: packet sequencing, ACK-based retransmission, fragment reassembly

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Packet types for UDP protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// Data packet (contains message payload)
    Data = 0x01,
    /// Acknowledgment of received packet
    Ack = 0x02,
    /// Keep-alive ping
    Ping = 0x03,
    /// Ping response
    Pong = 0x04,
}

impl PacketType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(PacketType::Data),
            0x02 => Some(PacketType::Ack),
            0x03 => Some(PacketType::Ping),
            0x04 => Some(PacketType::Pong),
            _ => None,
        }
    }
}

/// Packet header (13 bytes)
/// Type (1) + Sequence (4) + Ack (4) + Flags (4)
pub const PACKET_HEADER_SIZE: usize = 13;
pub const MAX_PACKET_SIZE: usize = 1280; // Conservative for UDP
pub const MAX_PAYLOAD_SIZE: usize = MAX_PACKET_SIZE - PACKET_HEADER_SIZE;

/// Fragment flags
pub const FRAG_FIRST: u32 = 0x02;
pub const FRAG_LAST: u32 = 0x01;

/// Represents a single UDP packet with header
#[derive(Clone)]
pub struct UdpPacket {
    pub packet_type: PacketType,
    pub sequence: u32,
    pub ack_number: u32,
    pub flags: u32,
    pub payload: Vec<u8>,
}

impl UdpPacket {
    /// Serialize packet to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(PACKET_HEADER_SIZE + self.payload.len());
        
        buf.push(self.packet_type as u8);
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.ack_number.to_be_bytes());
        buf.extend_from_slice(&self.flags.to_be_bytes());
        buf.extend_from_slice(&self.payload);
        
        buf
    }

    /// Deserialize packet from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < PACKET_HEADER_SIZE {
            return Err("Packet too small".to_string());
        }

        let packet_type = PacketType::from_u8(data[0])
            .ok_or_else(|| "Invalid packet type".to_string())?;
        
        let sequence = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);
        let ack_number = u32::from_be_bytes([data[5], data[6], data[7], data[8]]);
        let flags = u32::from_be_bytes([data[9], data[10], data[11], data[12]]);
        
        let payload = if data.len() > PACKET_HEADER_SIZE {
            data[PACKET_HEADER_SIZE..].to_vec()
        } else {
            Vec::new()
        };

        Ok(UdpPacket {
            packet_type,
            sequence,
            ack_number,
            flags,
            payload,
        })
    }

    pub fn is_first_fragment(&self) -> bool {
        (self.flags & FRAG_FIRST) != 0
    }

    pub fn is_last_fragment(&self) -> bool {
        (self.flags & FRAG_LAST) != 0
    }
}

/// Manages packet sequencing, ACKs, and retransmissions
pub struct PacketManager {
    sequence_number: u32,
    last_ack_received: u32,
    /// Pending packets waiting for ACK: sequence -> (packet, send_time, retries)
    pending_acks: HashMap<u32, (UdpPacket, Instant, u32)>,
    /// Received sequence numbers (for detecting duplicates)
    received_sequences: std::collections::HashSet<u32>,
    /// Configuration
    ack_timeout: Duration,
    max_retries: u32,
}

impl PacketManager {
    pub fn new() -> Self {
        Self {
            sequence_number: 0,
            last_ack_received: u32::MAX, // Start at max so first real ack is 0
            pending_acks: HashMap::new(),
            received_sequences: std::collections::HashSet::new(),
            ack_timeout: Duration::from_millis(100),
            max_retries: 3,
        }
    }

    pub fn with_config(ack_timeout: Duration, max_retries: u32) -> Self {
        let mut pm = Self::new();
        pm.ack_timeout = ack_timeout;
        pm.max_retries = max_retries;
        pm
    }

    /// Create data packets from a message buffer, handling fragmentation
    pub fn create_packets(&mut self, data: &[u8]) -> Vec<UdpPacket> {
        let mut packets = Vec::new();

        if data.len() <= MAX_PAYLOAD_SIZE {
            // Single packet
            let packet = UdpPacket {
                packet_type: PacketType::Data,
                sequence: self.sequence_number,
                ack_number: self.last_ack_received,
                flags: FRAG_FIRST | FRAG_LAST, // Both first and last
                payload: data.to_vec(),
            };
            self.pending_acks.insert(
                self.sequence_number,
                (packet.clone(), Instant::now(), 0),
            );
            packets.push(packet);
            self.sequence_number = self.sequence_number.wrapping_add(1);
        } else {
            // Multi-packet fragmentation
            let mut offset = 0;
            while offset < data.len() {
                let end = std::cmp::min(offset + MAX_PAYLOAD_SIZE, data.len());
                let chunk = &data[offset..end];
                
                let is_first = offset == 0;
                let is_last = end == data.len();
                let flags = if is_first { FRAG_FIRST } else { 0 }
                    | if is_last { FRAG_LAST } else { 0 };

                let packet = UdpPacket {
                    packet_type: PacketType::Data,
                    sequence: self.sequence_number,
                    ack_number: self.last_ack_received,
                    flags,
                    payload: chunk.to_vec(),
                };

                self.pending_acks.insert(
                    self.sequence_number,
                    (packet.clone(), Instant::now(), 0),
                );
                packets.push(packet);
                self.sequence_number = self.sequence_number.wrapping_add(1);
                offset = end;
            }
        }

        packets
    }

    /// Create an ACK packet
    pub fn create_ack_packet(&self, ack_sequence: u32) -> UdpPacket {
        UdpPacket {
            packet_type: PacketType::Ack,
            sequence: self.sequence_number,
            ack_number: ack_sequence,
            flags: 0,
            payload: Vec::new(),
        }
    }

    /// Create a PING packet
    pub fn create_ping_packet(&mut self) -> UdpPacket {
        let packet = UdpPacket {
            packet_type: PacketType::Ping,
            sequence: self.sequence_number,
            ack_number: self.last_ack_received,
            flags: 0,
            payload: Instant::now()
                .elapsed()
                .as_millis()
                .to_le_bytes()
                .to_vec(),
        };
        self.sequence_number = self.sequence_number.wrapping_add(1);
        packet
    }

    /// Record received packet to prevent duplicate processing
    pub fn record_received(&mut self, sequence: u32) -> bool {
        if self.received_sequences.contains(&sequence) {
            return false; // Duplicate
        }
        self.received_sequences.insert(sequence);
        self.last_ack_received = self.last_ack_received.max(sequence);
        true
    }

    /// Handle incoming ACK, returns true if it was for a pending packet
    pub fn handle_ack(&mut self, ack_number: u32) -> bool {
        self.pending_acks.remove(&ack_number).is_some()
    }

    /// Get packets that need retransmission due to timeout
    pub fn get_retransmit_packets(&mut self) -> Vec<(UdpPacket, u32)> {
        let now = Instant::now();
        let mut to_retransmit = Vec::new();
        let mut to_remove = Vec::new();

        for (&seq, (packet, send_time, retries)) in self.pending_acks.iter_mut() {
            if now.duration_since(*send_time) > self.ack_timeout {
                if *retries < self.max_retries {
                    // Retransmit
                    let retry_packet = packet.clone();
                    *send_time = now;
                    *retries += 1;
                    to_retransmit.push((retry_packet, *retries));
                } else {
                    // Max retries exceeded
                    to_remove.push(seq);
                }
            }
        }

        for seq in to_remove {
            self.pending_acks.remove(&seq);
        }

        to_retransmit
    }

    /// Check if there are pending ACKs
    pub fn has_pending_acks(&self) -> bool {
        !self.pending_acks.is_empty()
    }

    /// Get count of pending ACKs
    pub fn pending_ack_count(&self) -> usize {
        self.pending_acks.len()
    }

    /// Reset internal state (for new session)
    pub fn reset(&mut self) {
        self.sequence_number = 0;
        self.last_ack_received = u32::MAX;
        self.pending_acks.clear();
        self.received_sequences.clear();
    }
}

/// Handles reassembly of fragmented messages
pub struct FragmentReassembler {
    /// Incomplete messages: message_id -> fragments
    incomplete_messages: HashMap<u32, Vec<Option<Vec<u8>>>>,
}

impl FragmentReassembler {
    pub fn new() -> Self {
        Self {
            incomplete_messages: HashMap::new(),
        }
    }

    /// Add a fragment, returns complete message if all fragments received
    pub fn add_fragment(
        &mut self,
        message_id: u32,
        fragment_index: u32,
        _is_first: bool,
        _is_last: bool,
        data: Vec<u8>,
    ) -> Option<Vec<u8>> {
        let fragments = self.incomplete_messages
            .entry(message_id)
            .or_insert_with(Vec::new);

        let idx = fragment_index as usize;
        if idx >= fragments.len() {
            fragments.resize(idx + 1, None);
        }

        if fragments[idx].is_none() {
            fragments[idx] = Some(data);
        }

        // Check if complete
        if !fragments.is_empty() && fragments.iter().all(|f| f.is_some()) {
            let message = self.incomplete_messages.remove(&message_id).unwrap();
            let complete = message.into_iter()
                .filter_map(|f| f)
                .collect::<Vec<_>>()
                .concat();
            Some(complete)
        } else {
            None
        }
    }

    /// Clean old incomplete messages (timeout-based cleanup)
    pub fn cleanup(&mut self) {
        // In production, track timestamps and remove old incomplete messages
        // For now, just keep in memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization() {
        let packet = UdpPacket {
            packet_type: PacketType::Data,
            sequence: 123,
            ack_number: 456,
            flags: FRAG_FIRST | FRAG_LAST,
            payload: vec![1, 2, 3, 4],
        };

        let bytes = packet.to_bytes();
        let deserialized = UdpPacket::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.packet_type, PacketType::Data);
        assert_eq!(deserialized.sequence, 123);
        assert_eq!(deserialized.ack_number, 456);
        assert_eq!(deserialized.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_packet_manager_single_message() {
        let mut pm = PacketManager::new();
        let packets = pm.create_packets(&[1, 2, 3, 4, 5]);

        assert_eq!(packets.len(), 1);
        assert!(packets[0].is_first_fragment());
        assert!(packets[0].is_last_fragment());
        assert_eq!(pm.pending_ack_count(), 1);
    }

    #[test]
    fn test_packet_manager_fragmentation() {
        let mut pm = PacketManager::new();
        let large_data = vec![0u8; MAX_PAYLOAD_SIZE * 3 + 100];
        let packets = pm.create_packets(&large_data);

        assert!(packets.len() > 1);
        assert!(packets[0].is_first_fragment());
        assert!(!packets[0].is_last_fragment());
        assert!(!packets[packets.len() - 1].is_first_fragment());
        assert!(packets[packets.len() - 1].is_last_fragment());
        assert_eq!(pm.pending_ack_count(), packets.len());
    }

    #[test]
    fn test_ack_handling() {
        let mut pm = PacketManager::new();
        let packets = pm.create_packets(&[1, 2, 3]);
        let seq = packets[0].sequence;

        assert_eq!(pm.pending_ack_count(), 1);
        let handled = pm.handle_ack(seq);
        assert!(handled);
        assert_eq!(pm.pending_ack_count(), 0);
    }

    #[test]
    fn test_duplicate_detection() {
        let mut pm = PacketManager::new();
        
        let is_new = pm.record_received(42);
        assert!(is_new);
        
        let is_duplicate = pm.record_received(42);
        assert!(!is_duplicate);
    }
}
