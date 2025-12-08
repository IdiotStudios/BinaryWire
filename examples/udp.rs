//! BiWi UDP Example
//! Demonstrates UDP server and client with automatic packet loss recovery

use biwi::{BiWiMessage, BiWiValue, BiWiUdpServer, BiWiUdpClient};
use std::io;
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    println!("=== BiWi UDP Echo Server & Client Example ===\n");

    // Create UDP server on port 9001
    let mut server = BiWiUdpServer::new("127.0.0.1", 9001)?;
    
    println!("[Server] Listening for messages...");

    // Give server a moment to start
    thread::sleep(Duration::from_millis(100));

    // Create UDP client
    println!("[Client] Connecting to server...");
    let client = BiWiUdpClient::connect("127.0.0.1:9001")?;
    println!("[Client] Connected!\n");

    // Send and receive in a loop
    for i in 1..=3 {
        // Client sends message
        let mut msg = BiWiMessage::new();
        msg.set_field(1, BiWiValue::String(format!("Hello from client #{}", i)));
        msg.set_field(2, BiWiValue::Int32(i as i32));
        
        println!("[Client] Sending message {}...", i);
        client.send(&msg)?;
        
        // Server receives and echoes back
        thread::sleep(Duration::from_millis(50));
        if let Some((client_id, server_msg)) = server.recv_packet() {
            println!("[Server] Received from {}: {:?}", client_id, server_msg.get_field(1));
            
            // Echo back to client
            server.send_to(&client_id, &server_msg)?;
            println!("[Server] Echoed back to client");
        }
        
        // Client receives echo
        thread::sleep(Duration::from_millis(50));
        match client.try_recv() {
            Some(response) => {
                println!("[Client] Received echo: {:?}\n", response.get_field(1));
            }
            None => {
                println!("[Client] No echo received (packet may have been lost)\n");
            }
        }
    }

    println!("[Client] Disconnecting...");
    drop(client);
    
    println!("✓ Example completed successfully!");
    
    Ok(())
}

