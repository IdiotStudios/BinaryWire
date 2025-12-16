# BiWi - Binary Wire Protocol (Rust)

A streaming, binary-first alternative to JSON designed for low-latency, incremental data transmission. This is the Rust implementation of the BiWi protocol.

## Status

✅ **Base System Complete** - Core encoding/decoding functionality implemented
✅ **UDP Networking** - Fast UDP transport with automatic packet loss recovery

### Implemented Features

- ✅ Type system with all BiWi types (Null, Boolean, Int32, Int64, Float32, Float64, String, Binary, Array, Object, Chunks)
- ✅ Varint encoding for efficient integer representation
- ✅ BiWiEncoder for encoding values to binary format
- ✅ BiWiDecoder for decoding binary data to values
- ✅ BiWiMessage for convenient field-based message handling
- ✅ Buffer caching for efficient re-encoding
- ✅ Full test coverage
- ✅ **UDP Server** (`BiWiUdpServer`) - Synchronous server with packet loss recovery
- ✅ **UDP Client** (`BiWiUdpClient`) - Non-blocking client with automatic retransmission
- ✅ **Packet Manager** - Sequence tracking, ACKs, fragmentation, and duplicate detection

### Todo

- ⏳ Async/Tokio support
- ⏳ TCP fallback mode
- ⏳ TLS encryption
- ⏳ Compression
- ⏳ Full benchmark suite

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
biwi = { path = "../BiWi" }  # Update path as needed
```

## Quick Start

```rust
use biwi::{BiWiMessage, BiWiValue};
use std::collections::HashMap;

fn main() {
    // Create a message
    let mut msg = BiWiMessage::new();
    
    // Add fields
    msg.set_field(1, BiWiValue::String("Hello, BiWi!".to_string()));
    msg.set_field(2, BiWiValue::Int32(42));
    msg.set_field(3, BiWiValue::Float64(3.14159));
    
    // Encode to binary
    let buffer = msg.to_vec();
    println!("Encoded size: {} bytes", buffer.len());
    
    // Decode from binary
    let decoded = BiWiMessage::from_buffer(&buffer).unwrap();
    println!("Field 1: {:?}", decoded.get_field(1));
}
```

## UDP Networking (Fast Transport with Packet Loss Recovery)

BiWi now includes a high-performance UDP implementation with automatic packet loss recovery:

```rust
use biwi::{BiWiUdpServer, BiWiUdpClient, BiWiMessage, BiWiValue};

// Server
let mut server = BiWiUdpServer::new("127.0.0.1", 9001)?;
if let Some((client_id, msg)) = server.recv_packet() {
    println!("Received: {:?}", msg.get_field(1));
    // Echo back
    server.send_to(&client_id, &msg)?;
}

// Client
let client = BiWiUdpClient::connect("127.0.0.1:9001")?;
let mut msg = BiWiMessage::new();
msg.set_field(1, BiWiValue::String("Hello".to_string()));
client.send(&msg)?;

// Receive (non-blocking)
if let Some(response) = client.try_recv() {
    println!("Response: {:?}", response.get_field(1));
}
```

### Features

- **Packet loss recovery**: Automatic retransmission with exponential backoff
- **Fragmentation**: Large messages automatically split into MTU-sized packets
- **ACK system**: Cumulative acknowledgments prevent duplicate processing
- **Low latency**: No connection handshake, minimal overhead
- **Scalable**: Each client has independent packet manager

See [UDP_IMPLEMENTATION.md](UDP_IMPLEMENTATION.md) for detailed documentation.

## Running Examples

```bash
# Basic encoding/decoding example
cargo run --example basic

# UDP server/client example
cargo run --example udp

# Express-style server/client example
cd examples/express

# Terminal 1: Start the server
cargo run --release --bin server

# Terminal 2: Run the client
cargo run --release --bin client

# Run tests
cargo test

# Build the library
cargo build --release
```

## Architecture

### Core Modules

- **`types`** - BiWi type definitions and type checking utilities
- **`encoder`** - Binary encoder with varint optimization
- **`decoder`** - Binary decoder with error handling
- **`message`** - High-level message abstraction with field management

### BiWiValue Enum

The `BiWiValue` enum represents all possible BiWi values:

```rust
pub enum BiWiValue {
    Null,
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Binary(Vec<u8>),
    Array(Vec<BiWiValue>),
    Object(HashMap<String, BiWiValue>),
}
```

### Performance Features

- **Varint encoding**: Optimized for small integers (< 128 uses 1 byte)
- **Buffer pre-allocation**: Reduces allocations during encoding
- **Message caching**: Encoded buffers are cached until fields change
- **Zero-copy decoding**: Decodes directly from byte slices

## Comparison with JavaScript Implementation

| Feature | JavaScript | Rust |
|---------|-----------|------|
| Type Safety | Runtime | Compile-time |
| Memory Safety | Garbage collected | Zero-cost abstractions |
| Performance | ~84k msgs/sec | TBD (benchmarks pending) |
| Error Handling | Exceptions | Result types |
| Async Support | Native | Tokio (pending) |

## Protocol Overview

### Message Format

Each BiWi message consists of one or more **fields**:

```
[Field 1][Field 2][Field 3]...
  │       │       │
  ├─→ [fieldId: varint][fieldType: 1 byte][value]
  ├─→ [fieldId: varint][fieldType: 1 byte][value]
  └─→ [fieldId: varint][fieldType: 1 byte][value]
```

### Supported Types

- **NULL** (0x00) - Null value
- **BOOLEAN** (0x01/0xFF) - True (0x01) / False (0xFF)
- **INT32** (0x02) - 32-bit signed integer (big-endian)
- **INT64** (0x03) - 64-bit signed integer (big-endian)
- **FLOAT32** (0x04) - Single-precision float (big-endian)
- **FLOAT64** (0x05) - Double-precision float (big-endian)
- **STRING** (0x06) - UTF-8 string with varint length
- **BINARY** (0x07) - Raw binary data with varint length
- **ARRAY** (0x08) - Ordered collection with varint count
- **OBJECT** (0x09) - Key-value mapping with varint count
- **CHUNK_START** (0x0A) - Begin streaming chunk
- **CHUNK_DATA** (0x0B) - Chunk payload
- **CHUNK_END** (0x0C) - End streaming

## Contributing

Contributions are welcome! This is a step-by-step conversion from the JavaScript implementation.
