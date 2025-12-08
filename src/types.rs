// BiWi Type Definitions
// Defines the types supported by the BiWi protocol

// BiWi protocol type codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BiWiType {
    Null = 0x00,
    Boolean = 0x01,
    Int32 = 0x02,
    Int64 = 0x03,
    Float32 = 0x04,
    Float64 = 0x05,
    String = 0x06,
    Binary = 0x07,
    Array = 0x08,
    Object = 0x09,
    ChunkStart = 0x0A,
    ChunkData = 0x0B,
    ChunkEnd = 0x0C,
}

impl BiWiType {
    /// Convert a u8 type code to BiWiType
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(BiWiType::Null),
            0x01 => Some(BiWiType::Boolean),
            0x02 => Some(BiWiType::Int32),
            0x03 => Some(BiWiType::Int64),
            0x04 => Some(BiWiType::Float32),
            0x05 => Some(BiWiType::Float64),
            0x06 => Some(BiWiType::String),
            0x07 => Some(BiWiType::Binary),
            0x08 => Some(BiWiType::Array),
            0x09 => Some(BiWiType::Object),
            0x0A => Some(BiWiType::ChunkStart),
            0x0B => Some(BiWiType::ChunkData),
            0x0C => Some(BiWiType::ChunkEnd),
            _ => None,
        }
    }

    /// Get the name of the type
    pub fn name(&self) -> &'static str {
        match self {
            BiWiType::Null => "NULL",
            BiWiType::Boolean => "BOOLEAN",
            BiWiType::Int32 => "INT32",
            BiWiType::Int64 => "INT64",
            BiWiType::Float32 => "FLOAT32",
            BiWiType::Float64 => "FLOAT64",
            BiWiType::String => "STRING",
            BiWiType::Binary => "BINARY",
            BiWiType::Array => "ARRAY",
            BiWiType::Object => "OBJECT",
            BiWiType::ChunkStart => "CHUNK_START",
            BiWiType::ChunkData => "CHUNK_DATA",
            BiWiType::ChunkEnd => "CHUNK_END",
        }
    }

    /// Check if this is a fixed-size type
    pub fn is_fixed_size(&self) -> bool {
        matches!(
            self,
            BiWiType::Null
                | BiWiType::Boolean
                | BiWiType::Int32
                | BiWiType::Int64
                | BiWiType::Float32
                | BiWiType::Float64
        )
    }

    /// Check if this is a variable-size type
    pub fn is_variable_size(&self) -> bool {
        matches!(
            self,
            BiWiType::String | BiWiType::Binary | BiWiType::Array | BiWiType::Object
        )
    }

    /// Check if this is a streaming type
    pub fn is_streaming_type(&self) -> bool {
        matches!(
            self,
            BiWiType::ChunkStart | BiWiType::ChunkData | BiWiType::ChunkEnd
        )
    }

    /// Get the fixed size in bytes (if applicable)
    pub fn fixed_size(&self) -> Option<usize> {
        match self {
            BiWiType::Null => Some(0),
            BiWiType::Boolean => Some(1),
            BiWiType::Int32 => Some(4),
            BiWiType::Int64 => Some(8),
            BiWiType::Float32 => Some(4),
            BiWiType::Float64 => Some(8),
            _ => None,
        }
    }
}

impl From<BiWiType> for u8 {
    fn from(t: BiWiType) -> u8 {
        t as u8
    }
}
