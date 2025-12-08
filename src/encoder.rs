//! BiWi Binary Encoder - Optimized for Performance & Efficiency
//! Encodes Rust values into BiWi binary format with compression techniques

use crate::types::BiWiType;
use std::collections::HashMap;

/// BiWi value representation with inlined small values for allocation efficiency
#[derive(Debug, Clone, PartialEq)]
pub enum BiWiValue {
    Null,
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    /// Small string (≤15 bytes) inlined, no allocation
    SmallString(SmallString),
    /// Large string (>15 bytes) allocated
    String(String),
    Binary(Vec<u8>),
    Array(Vec<BiWiValue>),
    Object(HashMap<String, BiWiValue>),
}

/// Inline small string (up to 15 bytes with 1-byte length)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmallString {
    len: u8,
    bytes: [u8; 15],
}

impl SmallString {
    pub fn new(s: &str) -> Option<Self> {
        let bytes_slice = s.as_bytes();
        if bytes_slice.len() > 15 {
            return None;
        }
        let mut bytes = [0u8; 15];
        bytes[..bytes_slice.len()].copy_from_slice(bytes_slice);
        Some(SmallString {
            len: bytes_slice.len() as u8,
            bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

impl BiWiValue {
    /// Check if value can be safely encoded as Float32
    fn can_use_float32(value: f64) -> bool {
        // Only use FLOAT32 for very simple decimals
        if value.abs() < 1e-5 || value.abs() > 1e6 {
            return false;
        }

        // Check precision loss
        let as_f32 = value as f32;
        let roundtrip = as_f32 as f64;

        if value == 0.0 {
            return true;
        }

        let relative_error = ((roundtrip - value) / value).abs();
        relative_error < 0.00001
    }

    /// Create a Number value, automatically choosing the best type
    pub fn number(value: f64) -> Self {
        if value.fract() == 0.0 {
            // It's an integer
            if value >= i32::MIN as f64 && value <= i32::MAX as f64 {
                BiWiValue::Int32(value as i32)
            } else if value >= i64::MIN as f64 && value <= i64::MAX as f64 {
                BiWiValue::Int64(value as i64)
            } else {
                BiWiValue::Float64(value)
            }
        } else {
            // It's a float
            if Self::can_use_float32(value) {
                BiWiValue::Float32(value as f32)
            } else {
                BiWiValue::Float64(value)
            }
        }
    }
}

impl From<bool> for BiWiValue {
    fn from(b: bool) -> Self {
        BiWiValue::Boolean(b)
    }
}

impl From<i32> for BiWiValue {
    fn from(n: i32) -> Self {
        BiWiValue::Int32(n)
    }
}

impl From<i64> for BiWiValue {
    fn from(n: i64) -> Self {
        BiWiValue::Int64(n)
    }
}

impl From<f32> for BiWiValue {
    fn from(f: f32) -> Self {
        BiWiValue::Float32(f)
    }
}

impl From<f64> for BiWiValue {
    fn from(f: f64) -> Self {
        BiWiValue::Float64(f)
    }
}

impl From<String> for BiWiValue {
    fn from(s: String) -> Self {
        if let Some(small) = SmallString::new(&s) {
            BiWiValue::SmallString(small)
        } else {
            BiWiValue::String(s)
        }
    }
}

impl From<&str> for BiWiValue {
    fn from(s: &str) -> Self {
        if let Some(small) = SmallString::new(s) {
            BiWiValue::SmallString(small)
        } else {
            BiWiValue::String(s.to_string())
        }
    }
}

impl From<Vec<u8>> for BiWiValue {
    fn from(data: Vec<u8>) -> Self {
        BiWiValue::Binary(data)
    }
}

impl From<&[u8]> for BiWiValue {
    fn from(data: &[u8]) -> Self {
        BiWiValue::Binary(data.to_vec())
    }
}

/// BiWi encoder for converting values to binary format
pub struct BiWiEncoder {
    buffer: Vec<u8>,
}

impl BiWiEncoder {
    /// Create a new encoder with initial capacity
    pub fn new() -> Self {
        Self::with_capacity(128)
    }

    /// Create a new encoder with specified initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Write a varint (variable-length integer) optimized for small values
    fn write_varint(&mut self, mut value: u32) {
        // Fast path for common small values (0-127)
        if value < 0x80 {
            self.buffer.push(value as u8);
        } else if value < 0x4000 {
            // 2-byte varint (common for counts up to ~16k)
            self.buffer.push((value & 0x7f) as u8 | 0x80);
            self.buffer.push((value >> 7) as u8 & 0x7f);
        } else {
            // Fallback for large values
            while value >= 0x80 {
                self.buffer.push((value & 0x7f) as u8 | 0x80);
                value >>= 7;
            }
            self.buffer.push(value as u8 & 0x7f);
        }
    }

    /// Write a 64-bit varint
    fn write_varint_u64(&mut self, mut value: u64) {
        // Fast path for common small values (0-127)
        if value < 0x80 {
            self.buffer.push((value & 0x7f) as u8);
        } else if value < 0x4000 {
            // 2-byte varint (common for counts up to ~16k)
            self.buffer.push(((value & 0x7f) as u8) | 0x80);
            self.buffer.push(((value >> 7) as u8) & 0x7f);
        } else {
            // Fallback for large values
            while value >= 0x80 {
                self.buffer.push((value & 0x7f) as u8 | 0x80);
                value >>= 7;
            }
            self.buffer.push((value & 0x7f) as u8);
        }
    }

    /// Encode a complete field with field header
    /// Compact format for fields 1-63: [field_id:6 + wire_type:2]
    /// Extended format for fields 64+: [field_id(varint) + wire_type:3]
    pub fn encode_field(&mut self, field_id: u32, value: &BiWiValue) {
        let wire_type = match value {
            BiWiValue::Int32(_) | BiWiValue::Int64(_) => 2, // varint
            BiWiValue::String(_) | BiWiValue::Binary(_) | BiWiValue::Array(_) | BiWiValue::Object(_) => 3,
            BiWiValue::Float32(_) => 0, // fixed32
            BiWiValue::Float64(_) => 1, // fixed64
            _ => 2, // default varint
        };

        if field_id > 0 && field_id <= 63 {
            // Compact encoding: single byte with field_id (6 bits) + wire_type (2 bits)
            self.buffer.push(((field_id as u8) << 2) | (wire_type & 0x3));
        } else {
            // Standard encoding for field IDs > 63
            self.write_varint_u64((field_id as u64) << 3 | (wire_type as u64));
        }
        self.encode_value(value);
    }

    /// Encode a raw value with its type
    pub fn encode_value(&mut self, value: &BiWiValue) {
        match value {
            BiWiValue::Null => {
                self.buffer.push(BiWiType::Null as u8);
            }
            BiWiValue::Boolean(b) => {
                // Compact boolean encoding: single byte for type + value
                if *b {
                    self.buffer.push(BiWiType::Boolean as u8);
                } else {
                    self.buffer.push(0xFF); // Special marker for false
                }
            }
            BiWiValue::Int32(n) => {
                self.buffer.push(BiWiType::Int32 as u8);
                // Optimize for small integers: use single byte with marker
                if *n >= -64 && *n <= 63 {
                    self.buffer.push(((*n as i8) as u8) | 0x80); // Mark as small
                } else {
                    // Use zigzag + varint for larger integers
                    let zigzag = ((n << 1) ^ (n >> 31)) as u32;
                    self.write_varint(zigzag);
                }
            }
            BiWiValue::Int64(n) => {
                self.buffer.push(BiWiType::Int64 as u8);
                // Optimize for small integers: use single byte
                if *n >= -64 && *n <= 63 {
                    self.buffer.push(((*n as i8) as u8) | 0x80); // Mark as small
                } else {
                    // Use zigzag + varint for larger integers
                    let zigzag = ((n << 1) ^ (n >> 63)) as u64;
                    self.write_varint_u64(zigzag);
                }
            }
            BiWiValue::Float32(f) => {
                self.buffer.push(BiWiType::Float32 as u8);
                self.buffer.extend_from_slice(&f.to_be_bytes());
            }
            BiWiValue::Float64(f) => {
                self.buffer.push(BiWiType::Float64 as u8);
                self.buffer.extend_from_slice(&f.to_be_bytes());
            }
            BiWiValue::SmallString(s) => {
                self.buffer.push(BiWiType::String as u8);
                self.buffer.push(0x80 | (s.len & 0x7F)); // Mark as small string with length
                self.buffer.extend_from_slice(s.as_bytes());
            }
            BiWiValue::String(s) => {
                self.encode_string(s);
            }
            BiWiValue::Binary(data) => {
                self.encode_binary(data);
            }
            BiWiValue::Array(items) => {
                self.encode_array(items);
            }
            BiWiValue::Object(map) => {
                self.encode_object(map);
            }
        }
    }

    /// Encode a string with length optimization
    fn encode_string(&mut self, s: &str) {
        self.buffer.push(BiWiType::String as u8);
        let bytes = s.as_bytes();
        let length = bytes.len() as u32;

        // Write length as varint
        if length < 128 {
            self.buffer.push(length as u8);
        } else if length < 0x4000 {
            self.buffer.push((length & 0x7f) as u8 | 0x80);
            self.buffer.push((length >> 7) as u8 & 0x7f);
        } else {
            self.write_varint(length);
        }

        self.buffer.extend_from_slice(bytes);
    }

    /// Encode binary data with length optimization
    fn encode_binary(&mut self, data: &[u8]) {
        self.buffer.push(BiWiType::Binary as u8);
        let length = data.len() as u32;

        // Write length as varint
        self.write_varint(length);
        self.buffer.extend_from_slice(data);
    }

    /// Encode an array with packing optimization for primitive arrays
    fn encode_array(&mut self, items: &[BiWiValue]) {
        // Check if array contains only primitives of same type for packing
        if !items.is_empty() {
            let first_type = std::mem::discriminant(&items[0]);
            if items.iter().all(|item| std::mem::discriminant(item) == first_type) {
                // Homogeneous array - check if it's a primitive type that can be packed
                match &items[0] {
                    BiWiValue::Int32(_) | BiWiValue::Int64(_) | BiWiValue::Float32(_) | BiWiValue::Float64(_) => {
                        return self.encode_packed_array(items);
                    }
                    _ => {}
                }
            }
        }

        // Standard array encoding for heterogeneous or complex types
        self.buffer.push(BiWiType::Array as u8);
        let len = items.len() as u32;

        // Optimize for common case of small arrays (< 128 items)
        if len < 128 {
            self.buffer.push(len as u8);
        } else {
            self.write_varint(len);
        }

        for item in items {
            self.encode_value(item);
        }
    }

    /// Encode a packed array of primitives (no per-element type markers)
    fn encode_packed_array(&mut self, items: &[BiWiValue]) {
        // Packed format: [ARRAY_PACKED_TYPE][element_count][element_data...]
        // This saves one type byte per element
        
        let packed_type = match &items[0] {
            BiWiValue::Int32(_) => BiWiType::Int32,
            BiWiValue::Int64(_) => BiWiType::Int64,
            BiWiValue::Float32(_) => BiWiType::Float32,
            BiWiValue::Float64(_) => BiWiType::Float64,
            _ => unreachable!(),
        };

        // Mark as packed array: use high bit of type byte
        self.buffer.push(BiWiType::Array as u8 | 0x80); // High bit = packed
        self.buffer.push(packed_type as u8);

        let len = items.len() as u32;
        if len < 128 {
            self.buffer.push(len as u8);
        } else {
            self.write_varint(len);
        }

        // Encode elements without type markers
        for item in items {
            match item {
                BiWiValue::Int32(n) => {
                    let zigzag = ((n << 1) ^ (n >> 31)) as u32;
                    self.write_varint(zigzag);
                }
                BiWiValue::Int64(n) => {
                    let zigzag = ((n << 1) ^ (n >> 63)) as u64;
                    self.write_varint_u64(zigzag);
                }
                BiWiValue::Float32(f) => {
                    self.buffer.extend_from_slice(&f.to_be_bytes());
                }
                BiWiValue::Float64(f) => {
                    self.buffer.extend_from_slice(&f.to_be_bytes());
                }
                _ => unreachable!(),
            }
        }
    }

    /// Encode an object with key count optimization
    fn encode_object(&mut self, map: &HashMap<String, BiWiValue>) {
        self.buffer.push(BiWiType::Object as u8);
        let key_count = map.len() as u32;

        // Optimize for common case of small objects (< 128 keys)
        if key_count < 128 {
            self.buffer.push(key_count as u8);
        } else {
            self.write_varint(key_count);
        }

        for (key, value) in map {
            // Encode key as string (length + bytes)
            let key_bytes = key.as_bytes();
            let key_length = key_bytes.len() as u32;

            // Optimize for common case of short keys (< 128 bytes)
            if key_length < 128 {
                self.buffer.push(key_length as u8);
            } else {
                self.write_varint(key_length);
            }

            self.buffer.extend_from_slice(key_bytes);

            // Encode value
            self.encode_value(value);
        }
    }

    /// Encode a streaming chunk start
    pub fn encode_chunk_start(&mut self, field_id: u16, total_size: u32) {
        self.buffer.push(BiWiType::ChunkStart as u8);
        self.buffer.extend_from_slice(&field_id.to_be_bytes());
        self.buffer.extend_from_slice(&total_size.to_be_bytes());
    }

    /// Encode a streaming chunk data
    pub fn encode_chunk_data(&mut self, chunk_index: u16, data: &[u8]) {
        self.buffer.push(BiWiType::ChunkData as u8);
        self.buffer.extend_from_slice(&chunk_index.to_be_bytes());
        let data_length = data.len() as u16;
        self.buffer.extend_from_slice(&data_length.to_be_bytes());
        self.buffer.extend_from_slice(data);
    }

    /// Encode a streaming chunk end
    pub fn encode_chunk_end(&mut self) {
        self.buffer.push(BiWiType::ChunkEnd as u8);
    }

    /// Get the final buffer (consumes the encoder)
    pub fn to_buffer(self) -> Vec<u8> {
        self.buffer
    }

    /// Get a reference to the buffer without consuming
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the current size of the encoded data
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// Clear the buffer for reuse
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

impl Default for BiWiEncoder {
    fn default() -> Self {
        Self::new()
    }
}
