//! BiWi Binary Encoder - Optimized for Performance & Efficiency
//! Encodes Rust values into BiWi binary format with compression techniques

use crate::types::BiWiType;
use std::collections::HashMap;

/// BiWi value representation
#[derive(Debug, Clone, PartialEq)]
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
        BiWiValue::String(s)
    }
}

impl From<&str> for BiWiValue {
    fn from(s: &str) -> Self {
        BiWiValue::String(s.to_string())
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

    /// Encode a complete field with field header
    /// Format: [fieldId(varint)][value]
    pub fn encode_field(&mut self, field_id: u32, value: &BiWiValue) {
        self.write_varint(field_id);
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
                self.buffer.extend_from_slice(&n.to_be_bytes());
            }
            BiWiValue::Int64(n) => {
                self.buffer.push(BiWiType::Int64 as u8);
                self.buffer.extend_from_slice(&n.to_be_bytes());
            }
            BiWiValue::Float32(f) => {
                self.buffer.push(BiWiType::Float32 as u8);
                self.buffer.extend_from_slice(&f.to_be_bytes());
            }
            BiWiValue::Float64(f) => {
                self.buffer.push(BiWiType::Float64 as u8);
                self.buffer.extend_from_slice(&f.to_be_bytes());
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

    /// Encode an array with count optimization
    fn encode_array(&mut self, items: &[BiWiValue]) {
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
