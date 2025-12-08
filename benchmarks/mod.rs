use serde_json::json;

pub mod biwi;
pub mod json;
pub mod protobuf;
pub mod udp;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

#[derive(Clone)]
pub struct Scenario {
    pub name: &'static str,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug)]
pub struct StatResult {
    pub scenario: String,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub size_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct ThroughputResult {
    pub label: &'static str,
    pub throughput: f64,
    pub total_time_ms: f64,
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "Game State Update",
            payload: json!({
                "playerId": "player_12345",
                "position": {"x": 123.456, "y": 789.012, "z": 45.678},
                "rotation": {"pitch": 45.5, "yaw": 180.0, "roll": 0.0},
                "health": 95,
                "mana": 120,
                "stamina": 85,
                "equipment": ["sword", "shield", "boots"],
                "status": "running",
                "timestamp": now_ms(),
            }),
        },
        Scenario {
            name: "API Response (User Data)",
            payload: json!({
                "id": "user_98765",
                "username": "alice_wonderland",
                "email": "alice@example.com",
                "profile": {
                    "displayName": "Alice W.",
                    "avatar": "https://example.com/avatar.jpg",
                    "bio": "Software engineer and gaming enthusiast",
                    "followers": 1523,
                    "following": 342
                },
                "preferences": {
                    "notifications": true,
                    "theme": "dark",
                    "language": "en-US"
                },
                "verified": true,
                "createdAt": "2020-05-15T10:30:00Z",
                "lastLogin": now_ms(),
            }),
        },
        Scenario {
            name: "IoT Sensor Data",
            payload: json!({
                "deviceId": "sensor_001",
                "location": "warehouse_b",
                "readings": [
                    {"timestamp": now_ms() - 3000, "temperature": 22.5, "humidity": 45.2},
                    {"timestamp": now_ms() - 2000, "temperature": 22.6, "humidity": 45.1},
                    {"timestamp": now_ms() - 1000, "temperature": 22.4, "humidity": 45.3},
                    {"timestamp": now_ms(), "temperature": 22.5, "humidity": 45.2}
                ],
                "status": "healthy",
                "batteryLevel": 87,
                "signalStrength": -42,
            }),
        },
        Scenario {
            name: "Chat Message",
            payload: json!({
                "messageId": "msg_550e8400",
                "conversationId": "conv_12345",
                "senderId": "user_001",
                "senderName": "Bob",
                "text": "Hey Alice! Did you see the new game update? The graphics are amazing!",
                "timestamp": now_ms(),
                "reactions": {"👍": 5, "❤️": 3, "😂": 1},
                "mentions": ["Alice", "Charlie"],
                "edited": false
            }),
        },
        Scenario {
            name: "Stock Market Tick",
            payload: json!({
                "symbol": "AAPL",
                "price": 182.45,
                "priceChange": 1.23,
                "percentChange": 0.68,
                "volume": 52345600,
                "bid": 182.40,
                "ask": 182.50,
                "high": 183.50,
                "low": 181.20,
                "open": 181.22,
                "close": 182.45,
                "timestamp": now_ms(),
                "exchange": "NASDAQ",
            }),
        },
    ]
}

pub fn calc_stats(mut samples: Vec<f64>) -> (f64, f64, f64, f64, f64) {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = samples.len();
    let avg = samples.iter().sum::<f64>() / len as f64;
    let min = samples.first().copied().unwrap_or(0.0);
    let max = samples.last().copied().unwrap_or(0.0);
    let idx95 = ((len as f64) * 0.95).floor() as usize;
    let idx99 = ((len as f64) * 0.99).floor() as usize;
    let p95 = samples[idx95.min(len - 1)];
    let p99 = samples[idx99.min(len - 1)];
    (avg, min, max, p95, p99)
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    now.as_millis() as i64
}
