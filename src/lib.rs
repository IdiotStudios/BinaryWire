//! BiWi - Binary Wire Protocol
//! A streaming, binary-first alternative to JSON designed for low-latency,
//! incremental data transmission. Optimized for real-time applications,
//! game networking, and microservices.

// Core modules
pub mod types;
pub mod encoder;
pub mod decoder;
pub mod message;
pub mod network;
pub mod server;
pub mod client;

// Re-exports for convenience
pub use types::BiWiType;
pub use encoder::{BiWiEncoder, BiWiValue};
pub use decoder::{BiWiDecoder, DecodeError, DecodeResult, DecodedField, ChunkStart, ChunkData};
pub use message::BiWiMessage;
pub use network::{PacketManager, UdpPacket, PacketType};
pub use server::BiWiUdpServer;
pub use client::BiWiUdpClient;

/// BiWi protocol version
pub const VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_encoding_decoding() {
        let mut msg = BiWiMessage::new();
        msg.set_field(1, BiWiValue::String("Hello".to_string()));
        msg.set_field(2, BiWiValue::Int32(42));

        let buffer = msg.to_vec();
        let decoded = BiWiMessage::from_buffer(&buffer).unwrap();

        assert_eq!(decoded.get_field(1), Some(&BiWiValue::String("Hello".to_string())));
        assert_eq!(decoded.get_field(2), Some(&BiWiValue::Int32(42)));
    }

    #[test]
    fn test_types() {
        assert!(BiWiType::Int32.is_fixed_size());
        assert!(BiWiType::String.is_variable_size());
        assert!(BiWiType::ChunkStart.is_streaming_type());
        assert_eq!(BiWiType::Float64.name(), "FLOAT64");
    }

    #[test]
    fn test_varint_encoding() {
        let mut encoder = BiWiEncoder::new();
        encoder.encode_field(1, &BiWiValue::Int32(100));
        encoder.encode_field(200, &BiWiValue::Boolean(true));

        let buffer = encoder.to_buffer();
        let mut decoder = BiWiDecoder::new(&buffer);

        let field1 = decoder.decode_field().unwrap();
        assert_eq!(field1.field_id, 1);
        assert_eq!(field1.value, BiWiValue::Int32(100));

        let field2 = decoder.decode_field().unwrap();
        assert_eq!(field2.field_id, 200);
        assert_eq!(field2.value, BiWiValue::Boolean(true));
    }
}
