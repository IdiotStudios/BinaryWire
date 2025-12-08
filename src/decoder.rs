// BiWi Binary Decoder
// Decodes BiWi binary format into Rust values

use crate::encoder::BiWiValue;
use crate::types::BiWiType;
use std::collections::HashMap;

/// Errors that can occur during decoding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    InsufficientData(&'static str),
    UnknownType(u8),
    InvalidData(&'static str),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InsufficientData(msg) => write!(f, "Insufficient data: {}", msg),
            DecodeError::UnknownType(code) => write!(f, "Unknown type code: 0x{:02x}", code),
            DecodeError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl std::error::Error for DecodeError {}

pub type DecodeResult<T> = Result<T, DecodeError>;

/// Represents a decoded field with its ID and value
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedField {
    pub field_id: u32,
    pub value: BiWiValue,
}

/// Represents chunk-related data
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkStart {
    pub field_id: u16,
    pub total_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkData {
    pub chunk_index: u16,
    pub data: Vec<u8>,
}

/// BiWi decoder for converting binary format to values
pub struct BiWiDecoder<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> BiWiDecoder<'a> {
    /// Create a new decoder from a byte slice
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    /// Decode varint (variable-length integer) optimized for common cases
    fn read_varint(&mut self) -> DecodeResult<u32> {
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("varint"));
        }

        let mut byte = self.buffer[self.offset];
        self.offset += 1;

        // Fast path for single byte (0-127)
        if (byte & 0x80) == 0 {
            return Ok(byte as u32);
        }

        let mut value = (byte & 0x7f) as u32;
        let mut shift = 7;

        // Read remaining bytes
        while self.offset < self.buffer.len() {
            byte = self.buffer[self.offset];
            self.offset += 1;

            value |= ((byte & 0x7f) as u32) << shift;

            if (byte & 0x80) == 0 {
                return Ok(value);
            }

            shift += 7;
        }

        Err(DecodeError::InsufficientData("varint continuation"))
    }

    /// Decode a 64-bit varint
    fn read_varint_u64(&mut self) -> DecodeResult<u64> {
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("varint64"));
        }

        let mut byte = self.buffer[self.offset];
        self.offset += 1;

        // Fast path for single byte (0-127)
        if (byte & 0x80) == 0 {
            return Ok(byte as u64);
        }

        let mut value = (byte & 0x7f) as u64;
        let mut shift = 7;

        // Read remaining bytes
        while self.offset < self.buffer.len() {
            byte = self.buffer[self.offset];
            self.offset += 1;

            value |= ((byte & 0x7f) as u64) << shift;

            if (byte & 0x80) == 0 {
                return Ok(value);
            }

            shift += 7;
        }

        Err(DecodeError::InsufficientData("varint64 continuation"))
    }

    /// ZigZag decode a u32 to i32
    fn zigzag_decode_i32(value: u32) -> i32 {
        ((value >> 1) as i32) ^ -(((value & 1) as i32))
    }

    /// ZigZag decode a u64 to i64
    fn zigzag_decode_i64(value: u64) -> i64 {
        ((value >> 1) as i64) ^ -(((value & 1) as i64))
    }

    /// Decode a field with its header (handles compact and extended formats)
    /// Compact format (fields 1-63): [field_id:6 + wire_type:2]
    /// Extended format (fields 64+): [field_id(varint) + wire_type:3]
    pub fn decode_field(&mut self) -> DecodeResult<DecodedField> {
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("field header"));
        }

        let header_byte = self.buffer[self.offset];
        self.offset += 1;

        // Check if this is compact encoding (fields 1-63 use single byte)
        let field_id = if header_byte < 0x80 {
            // Compact format: extract field_id from top 6 bits
            (header_byte >> 2) as u32
        } else if header_byte >= 0xC0 {
            // Extended format starts with continuation bytes, put byte back and read varint
            self.offset -= 1;
            (self.read_varint_u64()? >> 3) as u32
        } else {
            // Single extended byte field ID
            (header_byte as u32) >> 3
        };

        let value = self.decode_value()?;
        Ok(DecodedField { field_id, value })
    }

    /// Decode a value with its type
    pub fn decode_value(&mut self) -> DecodeResult<BiWiValue> {
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("type byte"));
        }

        let type_code = self.buffer[self.offset];
        self.offset += 1;

        // Check for packed array marker (high bit set)
        if type_code == (BiWiType::Array as u8 | 0x80) {
            return self.decode_packed_array();
        }

        match type_code {
            0x00 => Ok(BiWiValue::Null),
            0x01 => Ok(BiWiValue::Boolean(true)),
            0xFF => Ok(BiWiValue::Boolean(false)),
            0x02 => self.decode_int32(),
            0x03 => self.decode_int64(),
            0x04 => self.decode_float32(),
            0x05 => self.decode_float64(),
            0x06 => self.decode_string(),
            0x07 => self.decode_binary(),
            0x08 => self.decode_array(),
            0x09 => self.decode_object(),
            _ => Err(DecodeError::UnknownType(type_code)),
        }
    }

    /// Decode a 32-bit integer with ZigZag decoding
    fn decode_int32(&mut self) -> DecodeResult<BiWiValue> {
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("int32 value"));
        }

        let byte = self.buffer[self.offset];
        
        // Check for small integer marker (high bit set)
        if byte & 0x80 != 0 {
            self.offset += 1;
            let value = (byte as i8) as i32;
            Ok(BiWiValue::Int32(value))
        } else {
            // Varint encoded value
            self.offset -= 1; // Put byte back for varint reading
            let zigzag = self.read_varint()?;
            let value = Self::zigzag_decode_i32(zigzag);
            Ok(BiWiValue::Int32(value))
        }
    }

    /// Decode a 64-bit integer with ZigZag decoding
    fn decode_int64(&mut self) -> DecodeResult<BiWiValue> {
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("int64 value"));
        }

        let byte = self.buffer[self.offset];
        
        // Check for small integer marker (high bit set)
        if byte & 0x80 != 0 {
            self.offset += 1;
            let value = (byte as i8) as i64;
            Ok(BiWiValue::Int64(value))
        } else {
            // Varint encoded value
            self.offset -= 1; // Put byte back for varint reading
            let zigzag = self.read_varint_u64()?;
            let value = Self::zigzag_decode_i64(zigzag);
            Ok(BiWiValue::Int64(value))
        }
    }

    /// Decode a 32-bit float
    fn decode_float32(&mut self) -> DecodeResult<BiWiValue> {
        if self.offset + 4 > self.buffer.len() {
            return Err(DecodeError::InsufficientData("float32"));
        }

        let bytes = &self.buffer[self.offset..self.offset + 4];
        self.offset += 4;

        let value = f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        Ok(BiWiValue::Float32(value))
    }

    /// Decode a 64-bit float
    fn decode_float64(&mut self) -> DecodeResult<BiWiValue> {
        if self.offset + 8 > self.buffer.len() {
            return Err(DecodeError::InsufficientData("float64"));
        }

        let bytes = &self.buffer[self.offset..self.offset + 8];
        self.offset += 8;

        let value = f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        Ok(BiWiValue::Float64(value))
    }

    /// Decode a string (handles both small and large)
    fn decode_string(&mut self) -> DecodeResult<BiWiValue> {
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("string length"));
        }

        let len_byte = self.buffer[self.offset];
        self.offset += 1;

        // Check if small string marker (high bit set)
        if len_byte & 0x80 != 0 {
            // Small string: length encoded in low 7 bits
            let length = (len_byte & 0x7F) as usize;
            if self.offset + length > self.buffer.len() {
                return Err(DecodeError::InsufficientData("small string content"));
            }

            let bytes = &self.buffer[self.offset..self.offset + length];
            self.offset += length;

            let s = std::str::from_utf8(bytes)
                .map_err(|_| DecodeError::InvalidData("invalid UTF-8 in small string"))?;

            // Try to create SmallString, fallback to regular String if needed
            if let Some(small) = crate::encoder::SmallString::new(s) {
                Ok(BiWiValue::SmallString(small))
            } else {
                Ok(BiWiValue::String(s.to_string()))
            }
        } else {
            // Large string: length is a varint starting with len_byte
            self.offset -= 1; // Put byte back
            let length = self.read_varint()? as usize;

            if self.offset + length > self.buffer.len() {
                return Err(DecodeError::InsufficientData("string content"));
            }

            let bytes = &self.buffer[self.offset..self.offset + length];
            self.offset += length;

            let value = String::from_utf8(bytes.to_vec())
                .map_err(|_| DecodeError::InvalidData("invalid UTF-8"))?;

            Ok(BiWiValue::String(value))
        }
    }

    /// Decode binary data
    fn decode_binary(&mut self) -> DecodeResult<BiWiValue> {
        let length = self.read_varint()? as usize;

        if self.offset + length > self.buffer.len() {
            return Err(DecodeError::InsufficientData("binary content"));
        }

        let data = self.buffer[self.offset..self.offset + length].to_vec();
        self.offset += length;

        Ok(BiWiValue::Binary(data))
    }

    /// Decode an array
    fn decode_array(&mut self) -> DecodeResult<BiWiValue> {
        let count = self.read_varint()? as usize;

        let mut array = Vec::with_capacity(count);
        for _ in 0..count {
            array.push(self.decode_value()?);
        }

        Ok(BiWiValue::Array(array))
    }

    /// Decode an object
    fn decode_object(&mut self) -> DecodeResult<BiWiValue> {
        let count = self.read_varint()? as usize;

        let mut map = HashMap::with_capacity(count);
        for _ in 0..count {
            // Decode key
            let key_length = self.read_varint()? as usize;

            if self.offset + key_length > self.buffer.len() {
                return Err(DecodeError::InsufficientData("key content"));
            }

            let key_bytes = &self.buffer[self.offset..self.offset + key_length];
            self.offset += key_length;

            let key = String::from_utf8(key_bytes.to_vec())
                .map_err(|_| DecodeError::InvalidData("invalid key UTF-8"))?;

            // Decode value
            let value = self.decode_value()?;

            map.insert(key, value);
        }

        Ok(BiWiValue::Object(map))
    }

    /// Decode chunk start header
    pub fn decode_chunk_start(&mut self) -> DecodeResult<ChunkStart> {
        if self.offset + 6 > self.buffer.len() {
            return Err(DecodeError::InsufficientData("chunk start"));
        }

        let field_id_bytes = &self.buffer[self.offset..self.offset + 2];
        let field_id = u16::from_be_bytes([field_id_bytes[0], field_id_bytes[1]]);
        self.offset += 2;

        let total_size_bytes = &self.buffer[self.offset..self.offset + 4];
        let total_size = u32::from_be_bytes([
            total_size_bytes[0],
            total_size_bytes[1],
            total_size_bytes[2],
            total_size_bytes[3],
        ]);
        self.offset += 4;

        Ok(ChunkStart {
            field_id,
            total_size,
        })
    }

    /// Decode chunk data
    pub fn decode_chunk_data(&mut self) -> DecodeResult<ChunkData> {
        if self.offset + 4 > self.buffer.len() {
            return Err(DecodeError::InsufficientData("chunk data header"));
        }

        let chunk_index_bytes = &self.buffer[self.offset..self.offset + 2];
        let chunk_index = u16::from_be_bytes([chunk_index_bytes[0], chunk_index_bytes[1]]);
        self.offset += 2;

        let data_length_bytes = &self.buffer[self.offset..self.offset + 2];
        let data_length = u16::from_be_bytes([data_length_bytes[0], data_length_bytes[1]]);
        self.offset += 2;

        let data_length = data_length as usize;
        if self.offset + data_length > self.buffer.len() {
            return Err(DecodeError::InsufficientData("chunk content"));
        }

        let data = self.buffer[self.offset..self.offset + data_length].to_vec();
        self.offset += data_length;

        Ok(ChunkData { chunk_index, data })
    }

    /// Decode a packed array of primitives (no per-element type markers)
    fn decode_packed_array(&mut self) -> DecodeResult<BiWiValue> {
        // Read element type
        if self.offset >= self.buffer.len() {
            return Err(DecodeError::InsufficientData("packed array type"));
        }
        
        let element_type = self.buffer[self.offset];
        self.offset += 1;

        // Read element count
        let count = self.read_varint()? as usize;
        let mut array = Vec::with_capacity(count);

        // Decode elements based on type
        match element_type {
            0x02 => {
                // Int32 packed array
                for _ in 0..count {
                    let zigzag = self.read_varint()?;
                    array.push(BiWiValue::Int32(Self::zigzag_decode_i32(zigzag)));
                }
            }
            0x03 => {
                // Int64 packed array
                for _ in 0..count {
                    let zigzag = self.read_varint_u64()?;
                    array.push(BiWiValue::Int64(Self::zigzag_decode_i64(zigzag)));
                }
            }
            0x04 => {
                // Float32 packed array
                for _ in 0..count {
                    if self.offset + 4 > self.buffer.len() {
                        return Err(DecodeError::InsufficientData("float32 in packed array"));
                    }
                    let bytes = &self.buffer[self.offset..self.offset + 4];
                    self.offset += 4;
                    let value = f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    array.push(BiWiValue::Float32(value));
                }
            }
            0x05 => {
                // Float64 packed array
                for _ in 0..count {
                    if self.offset + 8 > self.buffer.len() {
                        return Err(DecodeError::InsufficientData("float64 in packed array"));
                    }
                    let bytes = &self.buffer[self.offset..self.offset + 8];
                    self.offset += 8;
                    let value = f64::from_be_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                    ]);
                    array.push(BiWiValue::Float64(value));
                }
            }
            _ => return Err(DecodeError::InvalidData("unknown packed array element type")),
        }

        Ok(BiWiValue::Array(array))
    }

    /// Decode all fields in the buffer
    pub fn decode_all(&mut self) -> Vec<DecodedField> {
        let mut fields = Vec::new();
        while self.offset < self.buffer.len() {
            match self.decode_field() {
                Ok(field) => fields.push(field),
                Err(_) => break, // Stop on incomplete data
            }
        }
        fields
    }

    /// Check if there's more data to decode
    pub fn has_more(&self) -> bool {
        self.offset < self.buffer.len()
    }

    /// Get remaining bytes count
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.offset
    }

    /// Get current offset
    pub fn offset(&self) -> usize {
        self.offset
    }
}
