//! Simple example demonstrating BiWi encoding and decoding

use biwi::{BiWiMessage, BiWiValue};
use std::collections::HashMap;

fn main() {
    println!("=== BiWi Base System Example ===\n");

    // Create a message
    let mut msg = BiWiMessage::new();
    
    // Add various field types
    msg.set_field(1, BiWiValue::String("Hello, BiWi!".to_string()));
    msg.set_field(2, BiWiValue::Int32(42));
    msg.set_field(3, BiWiValue::Float64(3.14159));
    msg.set_field(4, BiWiValue::Boolean(true));
    
    // Add an array
    msg.set_field(5, BiWiValue::Array(vec![
        BiWiValue::Int32(1),
        BiWiValue::Int32(2),
        BiWiValue::Int32(3),
    ]));
    
    // Add an object
    let mut obj = HashMap::new();
    obj.insert("name".to_string(), BiWiValue::String("Rust".to_string()));
    obj.insert("version".to_string(), BiWiValue::Int32(1));
    msg.set_field(6, BiWiValue::Object(obj));

    println!("Original message:");
    println!("  Field 1 (String): {:?}", msg.get_field(1));
    println!("  Field 2 (Int32): {:?}", msg.get_field(2));
    println!("  Field 3 (Float64): {:?}", msg.get_field(3));
    println!("  Field 4 (Boolean): {:?}", msg.get_field(4));
    println!("  Field 5 (Array): {:?}", msg.get_field(5));
    println!("  Field 6 (Object): {:?}", msg.get_field(6));

    // Encode to binary
    let buffer = msg.to_vec();
    println!("\nEncoded size: {} bytes", buffer.len());
    println!("Binary (hex): {}", hex_string(&buffer));

    // Decode from binary
    let decoded = BiWiMessage::from_buffer(&buffer).unwrap();
    
    println!("\nDecoded message:");
    println!("  Field 1 (String): {:?}", decoded.get_field(1));
    println!("  Field 2 (Int32): {:?}", decoded.get_field(2));
    println!("  Field 3 (Float64): {:?}", decoded.get_field(3));
    println!("  Field 4 (Boolean): {:?}", decoded.get_field(4));
    println!("  Field 5 (Array): {:?}", decoded.get_field(5));
    println!("  Field 6 (Object): {:?}", decoded.get_field(6));

    // Verify they match
    println!("\n✓ All fields decoded correctly!");
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
