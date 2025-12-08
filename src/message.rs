// BiWi Message
// Represents a complete BiWi message with fields

use crate::decoder::{BiWiDecoder, DecodeResult};
use crate::encoder::{BiWiEncoder, BiWiValue};
use std::collections::HashMap;

/// BiWi message containing multiple fields
pub struct BiWiMessage {
    fields: HashMap<u32, BiWiValue>,
    cached_buffer: Option<Vec<u8>>,
}

impl BiWiMessage {
    /// Create a new empty message
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            cached_buffer: None,
        }
    }

    /// Create a message with initial capacity for fields
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            fields: HashMap::with_capacity(capacity),
            cached_buffer: None,
        }
    }

    /// Set a field value (invalidates cache)
    pub fn set_field(&mut self, field_id: u32, value: BiWiValue) -> &mut Self {
        self.fields.insert(field_id, value);
        self.cached_buffer = None; // Invalidate cache
        self
    }

    /// Get a field value
    pub fn get_field(&self, field_id: u32) -> Option<&BiWiValue> {
        self.fields.get(&field_id)
    }

    /// Get a mutable reference to a field value
    pub fn get_field_mut(&mut self, field_id: u32) -> Option<&mut BiWiValue> {
        self.cached_buffer = None; // Invalidate cache since field might be modified
        self.fields.get_mut(&field_id)
    }

    /// Check if field exists
    pub fn has_field(&self, field_id: u32) -> bool {
        self.fields.contains_key(&field_id)
    }

    /// Remove a field
    pub fn remove_field(&mut self, field_id: u32) -> Option<BiWiValue> {
        self.cached_buffer = None; // Invalidate cache
        self.fields.remove(&field_id)
    }

    /// Get all field IDs
    pub fn field_ids(&self) -> Vec<u32> {
        self.fields.keys().copied().collect()
    }

    /// Get the number of fields
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Clear all fields
    pub fn clear(&mut self) {
        self.fields.clear();
        self.cached_buffer = None;
    }

    /// Get all fields as a HashMap reference
    pub fn fields(&self) -> &HashMap<u32, BiWiValue> {
        &self.fields
    }

    /// Encode message to binary (cached)
    pub fn to_buffer(&mut self) -> &[u8] {
        if let Some(ref buffer) = self.cached_buffer {
            return buffer;
        }

        let mut encoder = BiWiEncoder::new();
        for (field_id, value) in &self.fields {
            encoder.encode_field(*field_id, value);
        }

        let buffer = encoder.to_buffer();
        self.cached_buffer = Some(buffer);
        self.cached_buffer.as_ref().unwrap()
    }

    /// Encode message to a new Vec<u8> (doesn't cache)
    pub fn to_vec(&self) -> Vec<u8> {
        let mut encoder = BiWiEncoder::new();
        for (field_id, value) in &self.fields {
            encoder.encode_field(*field_id, value);
        }
        encoder.to_buffer()
    }

    /// Decode from binary buffer
    pub fn from_buffer(buffer: &[u8]) -> DecodeResult<Self> {
        let mut decoder = BiWiDecoder::new(buffer);
        let mut message = BiWiMessage::new();

        let fields = decoder.decode_all();
        for field in fields {
            message.set_field(field.field_id, field.value);
        }

        Ok(message)
    }

    /// Get size of encoded message
    pub fn size(&mut self) -> usize {
        self.to_buffer().len()
    }
}

impl Default for BiWiMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for BiWiMessage {
    fn clone(&self) -> Self {
        Self {
            fields: self.fields.clone(),
            cached_buffer: None, // Don't clone cache, will be regenerated if needed
        }
    }
}

impl std::fmt::Debug for BiWiMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BiWiMessage")
            .field("fields", &self.fields)
            .field("cached", &self.cached_buffer.is_some())
            .finish()
    }
}
