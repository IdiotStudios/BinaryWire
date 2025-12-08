use crate::benchmarks::{calc_stats, proto, scenarios, Scenario, StatResult, ThroughputResult};
use bytes::BytesMut;
use prost::Message;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Instant;

enum ProtoMsg {
    Game(proto::GameStateUpdate),
    Api(proto::ApiResponse),
    IoT(proto::IoTSensorData),
    Chat(proto::ChatMessage),
    Stock(proto::StockTick),
    Test(proto::TestMessage),
}

pub fn run_protobuf_benchmark() -> (Vec<StatResult>, ThroughputResult) {
    let pure = run_pure();
    let net = run_network();
    let mut combined = pure;
    combined.extend(net.0);
    (combined, net.1)
}

fn run_pure() -> Vec<StatResult> {
    let mut results = Vec::new();
    for scenario in scenarios() {
        let encode_samples = bench_encode(&scenario, 5_000);
        let decode_samples = bench_decode(&scenario, 5_000);
        let (enc_avg, enc_min, enc_max, enc_p95, enc_p99) = calc_stats(encode_samples);
        let (dec_avg, dec_min, dec_max, dec_p95, dec_p99) = calc_stats(decode_samples);

        let combined_avg = enc_avg + dec_avg;
        let combined_min = enc_min + dec_min;
        let combined_max = enc_max + dec_max;
        let combined_p95 = enc_p95 + dec_p95;
        let combined_p99 = enc_p99 + dec_p99;

        let size = encoded_size(&scenario);

        results.push(StatResult {
            scenario: format!("{} (pure)", scenario.name),
            avg_ms: combined_avg,
            min_ms: combined_min,
            max_ms: combined_max,
            p95_ms: combined_p95,
            p99_ms: combined_p99,
            size_bytes: size,
        });
    }
    results
}

fn run_network() -> (Vec<StatResult>, ThroughputResult) {
    let listener = TcpListener::bind("127.0.0.1:4012").expect("bind proto");
    let _server = thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                thread::spawn(move || handle_client(&mut s));
            }
        }
    });
    std::thread::sleep(std::time::Duration::from_millis(100));

    let scens = scenarios();
    let mut results = Vec::new();

    for scenario in &scens {
        let mut stream = TcpStream::connect("127.0.0.1:4012").expect("connect proto");
        let (avg, min, max, p95, p99) = bench_round_trip(&mut stream, scenario, 200);
        let size = encoded_size(scenario) + 4; // length prefix
        results.push(StatResult {
            scenario: format!("{} (net)", scenario.name),
            avg_ms: avg,
            min_ms: min,
            max_ms: max,
            p95_ms: p95,
            p99_ms: p99,
            size_bytes: size,
        });
    }

    let mut stream = TcpStream::connect("127.0.0.1:4012").expect("connect proto throughput");
    let throughput = throughput_test(&mut stream, 1_000);
    (results, throughput)
}

fn handle_client(stream: &mut TcpStream) {
    let mut buf = Vec::with_capacity(8 * 1024);
    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).is_err() {
            break;
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        buf.resize(len, 0);
        if stream.read_exact(&mut buf).is_err() {
            break;
        }
        // Echo back frame
        if stream.write_all(&len_buf).is_err() {
            break;
        }
        if stream.write_all(&buf).is_err() {
            break;
        }
    }
}

fn bench_encode(s: &Scenario, iterations: usize) -> Vec<f64> {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let proto = to_proto(s);
        let start = Instant::now();
        let _ = encode_proto(&proto);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples
}

fn bench_decode(s: &Scenario, iterations: usize) -> Vec<f64> {
    let proto = to_proto(s);
    let buf = encode_proto(&proto);
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = decode_proto(s.name, &buf);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples
}

fn bench_round_trip(stream: &mut TcpStream, s: &Scenario, iterations: usize) -> (f64, f64, f64, f64, f64) {
    let proto = to_proto(s);
    let buf = encode_proto(&proto);
    let len_bytes = (buf.len() as u32).to_be_bytes();
    let mut resp = Vec::new();
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        stream.write_all(&len_bytes).unwrap();
        stream.write_all(&buf).unwrap();

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).unwrap();
        let resp_len = u32::from_be_bytes(len_buf) as usize;
        resp.resize(resp_len, 0);
        stream.read_exact(&mut resp).unwrap();
        let _ = decode_proto(s.name, &resp);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    calc_stats(samples)
}

fn throughput_test(stream: &mut TcpStream, messages: usize) -> ThroughputResult {
    let mut received = 0usize;

    let start = Instant::now();
    for i in 0..messages {
        let buf = encode_proto(&ProtoMsg::Test(proto::TestMessage { test: "data".into(), index: i as i32 }));
        let len_bytes = (buf.len() as u32).to_be_bytes();
        stream.write_all(&len_bytes).unwrap();
        stream.write_all(&buf).unwrap();
    }

    let mut len_buf = [0u8; 4];
    let mut resp = Vec::new();
    while received < messages {
        if stream.read_exact(&mut len_buf).is_err() {
            break;
        }
        let resp_len = u32::from_be_bytes(len_buf) as usize;
        resp.resize(resp_len, 0);
        if stream.read_exact(&mut resp).is_err() {
            break;
        }
        received += 1;
    }

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let throughput = messages as f64 / (elapsed / 1000.0);
    ThroughputResult { label: "Protobuf", throughput, total_time_ms: elapsed }
}

fn encoded_size(s: &Scenario) -> usize {
    let proto = to_proto(s);
    encode_proto(&proto).len()
}

fn encode_proto(msg: &ProtoMsg) -> Vec<u8> {
    let mut buf = BytesMut::with_capacity(512);
    match msg {
        ProtoMsg::Game(m) => m.encode(&mut buf).unwrap(),
        ProtoMsg::Api(m) => m.encode(&mut buf).unwrap(),
        ProtoMsg::IoT(m) => m.encode(&mut buf).unwrap(),
        ProtoMsg::Chat(m) => m.encode(&mut buf).unwrap(),
        ProtoMsg::Stock(m) => m.encode(&mut buf).unwrap(),
        ProtoMsg::Test(m) => m.encode(&mut buf).unwrap(),
    }
    buf.to_vec()
}

fn decode_proto(name: &str, buf: &[u8]) -> ProtoMsg {
    match name {
        "Game State Update" => ProtoMsg::Game(proto::GameStateUpdate::decode(buf).unwrap()),
        "API Response (User Data)" => ProtoMsg::Api(proto::ApiResponse::decode(buf).unwrap()),
        "IoT Sensor Data" => ProtoMsg::IoT(proto::IoTSensorData::decode(buf).unwrap()),
        "Chat Message" => ProtoMsg::Chat(proto::ChatMessage::decode(buf).unwrap()),
        "Stock Market Tick" => ProtoMsg::Stock(proto::StockTick::decode(buf).unwrap()),
        _ => ProtoMsg::Test(proto::TestMessage::decode(buf).unwrap()),
    }
}

fn to_proto(s: &Scenario) -> ProtoMsg {
    match s.name {
        "Game State Update" => ProtoMsg::Game(build_game(s)),
        "API Response (User Data)" => ProtoMsg::Api(build_api(s)),
        "IoT Sensor Data" => ProtoMsg::IoT(build_iot(s)),
        "Chat Message" => ProtoMsg::Chat(build_chat(s)),
        "Stock Market Tick" => ProtoMsg::Stock(build_stock(s)),
        _ => ProtoMsg::Test(proto::TestMessage { test: "data".into(), index: 0 }),
    }
}

fn build_game(s: &Scenario) -> proto::GameStateUpdate {
    let p = &s.payload;
    let equipment: Vec<String> = p["equipment"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    proto::GameStateUpdate {
        player_id: p["playerId"].as_str().unwrap_or_default().to_string(),
        position: Some(proto::Position {
            x: p["position"]["x"].as_f64().unwrap_or(0.0) as f32,
            y: p["position"]["y"].as_f64().unwrap_or(0.0) as f32,
            z: p["position"]["z"].as_f64().unwrap_or(0.0) as f32,
        }),
        rotation: Some(proto::Rotation {
            pitch: p["rotation"]["pitch"].as_f64().unwrap_or(0.0) as f32,
            yaw: p["rotation"]["yaw"].as_f64().unwrap_or(0.0) as f32,
            roll: p["rotation"]["roll"].as_f64().unwrap_or(0.0) as f32,
        }),
        health: p["health"].as_i64().unwrap_or(0) as i32,
        mana: p["mana"].as_i64().unwrap_or(0) as i32,
        stamina: p["stamina"].as_i64().unwrap_or(0) as i32,
        equipment,
        status: p["status"].as_str().unwrap_or_default().to_string(),
        timestamp: p["timestamp"].as_i64().unwrap_or(0),
    }
}

fn build_api(s: &Scenario) -> proto::ApiResponse {
    let p = &s.payload;
    let profile = &p["profile"];
    let prefs = &p["preferences"];
    proto::ApiResponse {
        id: p["id"].as_str().unwrap_or_default().to_string(),
        username: p["username"].as_str().unwrap_or_default().to_string(),
        email: p["email"].as_str().unwrap_or_default().to_string(),
        profile: Some(proto::Profile {
            display_name: profile["displayName"].as_str().unwrap_or_default().to_string(),
            avatar: profile["avatar"].as_str().unwrap_or_default().to_string(),
            bio: profile["bio"].as_str().unwrap_or_default().to_string(),
            followers: profile["followers"].as_i64().unwrap_or(0) as i32,
            following: profile["following"].as_i64().unwrap_or(0) as i32,
        }),
        preferences: Some(proto::Preferences {
            notifications: prefs["notifications"].as_bool().unwrap_or(false),
            theme: prefs["theme"].as_str().unwrap_or_default().to_string(),
            language: prefs["language"].as_str().unwrap_or_default().to_string(),
        }),
        verified: p["verified"].as_bool().unwrap_or(false),
        created_at: p["createdAt"].as_str().unwrap_or_default().to_string(),
        last_login: p["lastLogin"].as_i64().unwrap_or(0),
    }
}

fn build_iot(s: &Scenario) -> proto::IoTSensorData {
    let p = &s.payload;
    let readings = p["readings"].as_array().map(|arr| {
        arr
            .iter()
            .map(|r| proto::Reading {
                timestamp: r["timestamp"].as_i64().unwrap_or(0),
                temperature: r["temperature"].as_f64().unwrap_or(0.0) as f32,
                humidity: r["humidity"].as_f64().unwrap_or(0.0) as f32,
            })
            .collect()
    }).unwrap_or_default();
    proto::IoTSensorData {
        device_id: p["deviceId"].as_str().unwrap_or_default().to_string(),
        location: p["location"].as_str().unwrap_or_default().to_string(),
        readings,
        status: p["status"].as_str().unwrap_or_default().to_string(),
        battery_level: p["batteryLevel"].as_i64().unwrap_or(0) as i32,
        signal_strength: p["signalStrength"].as_i64().unwrap_or(0) as i32,
    }
}

fn build_chat(s: &Scenario) -> proto::ChatMessage {
    let p = &s.payload;
    let reactions_map = p["reactions"].as_object();
    let mut reactions = BTreeMap::new();
    if let Some(map) = reactions_map {
        for (k, v) in map.iter() {
            if let Some(n) = v.as_i64() {
                reactions.insert(k.clone(), n as i32);
            }
        }
    }
    let mentions = p["mentions"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    proto::ChatMessage {
        message_id: p["messageId"].as_str().unwrap_or_default().to_string(),
        conversation_id: p["conversationId"].as_str().unwrap_or_default().to_string(),
        sender_id: p["senderId"].as_str().unwrap_or_default().to_string(),
        sender_name: p["senderName"].as_str().unwrap_or_default().to_string(),
        text: p["text"].as_str().unwrap_or_default().to_string(),
        timestamp: p["timestamp"].as_i64().unwrap_or(0),
        reactions,
        mentions,
        edited: p["edited"].as_bool().unwrap_or(false),
    }
}

fn build_stock(s: &Scenario) -> proto::StockTick {
    let p = &s.payload;
    proto::StockTick {
        symbol: p["symbol"].as_str().unwrap_or_default().to_string(),
        price: p["price"].as_f64().unwrap_or(0.0),
        price_change: p["priceChange"].as_f64().unwrap_or(0.0) as f32,
        percent_change: p["percentChange"].as_f64().unwrap_or(0.0) as f32,
        volume: p["volume"].as_i64().unwrap_or(0),
        bid: p["bid"].as_f64().unwrap_or(0.0) as f32,
        ask: p["ask"].as_f64().unwrap_or(0.0) as f32,
        high: p["high"].as_f64().unwrap_or(0.0) as f32,
        low: p["low"].as_f64().unwrap_or(0.0) as f32,
        open: p["open"].as_f64().unwrap_or(0.0) as f32,
        close: p["close"].as_f64().unwrap_or(0.0) as f32,
        timestamp: p["timestamp"].as_i64().unwrap_or(0),
        exchange: p["exchange"].as_str().unwrap_or_default().to_string(),
    }
}
