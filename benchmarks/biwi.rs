use biwi::{BiWiMessage, BiWiValue};
use crate::benchmarks::{calc_stats, scenarios, Scenario, StatResult, ThroughputResult};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Instant;

pub fn run_biwi_benchmark() -> (Vec<StatResult>, ThroughputResult) {
    let pure = run_pure();
    let net = run_network();
    let mut combined = pure;
    combined.extend(net.0);
    (combined, net.1)
}

fn run_pure() -> Vec<StatResult> {
    let scenarios = scenarios();
    let mut results = Vec::new();

    for scenario in scenarios {
        let encode_samples = benchmark_encode(&scenario, 5_000);
        let decode_samples = benchmark_decode(&scenario, 5_000);
        let (enc_avg, enc_min, enc_max, enc_p95, enc_p99) = calc_stats(encode_samples);
        let (dec_avg, dec_min, dec_max, dec_p95, dec_p99) = calc_stats(decode_samples);

        // For summary, average encode+decode
        let combined_avg = enc_avg + dec_avg;
        let combined_min = enc_min + dec_min;
        let combined_max = enc_max + dec_max;
        let combined_p95 = enc_p95 + dec_p95;
        let combined_p99 = enc_p99 + dec_p99;

        // Size
        let size = message_size(&scenario);

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
    let scenarios = scenarios();
    let listener = TcpListener::bind("127.0.0.1:4010").expect("bind biwi");

    // Echo server thread
    let _server = thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                stream.set_nodelay(true).ok();
                thread::spawn(move || handle_client(&mut stream));
            }
        }
    });

    // Give server a moment
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut results = Vec::new();

    for scenario in &scenarios {
        let mut stream = TcpStream::connect("127.0.0.1:4010").expect("connect biwi");
        stream.set_nodelay(true).expect("set_nodelay");
        let (avg, min, max, p95, p99) = benchmark_round_trip(&mut stream, scenario, 200);
        let size = message_size(scenario) + 4; // length prefix
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

    // Throughput test
    let mut stream = TcpStream::connect("127.0.0.1:4010").expect("connect biwi throughput");
    stream.set_nodelay(true).expect("set_nodelay");
    let throughput = throughput_test(&mut stream, 1_000);

    (results, throughput)
}

fn handle_client(stream: &mut TcpStream) {
    let mut buf = Vec::with_capacity(8 * 1024);
    loop {
        let mut len_buf = [0u8; 4];
        if let Err(_) = stream.read_exact(&mut len_buf) {
            break;
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        buf.resize(len, 0);
        if let Err(_) = stream.read_exact(&mut buf) {
            break;
        }

        // Decode then echo back same payload
        let _ = BiWiMessage::from_buffer(&buf);
        let mut frame = Vec::with_capacity(4 + len);
        frame.extend_from_slice(&(len as u32).to_be_bytes());
        frame.extend_from_slice(&buf);
        if let Err(_) = stream.write_all(&frame) {
            break;
        }
        let _ = stream.flush();
    }
}

fn benchmark_encode(s: &Scenario, iterations: usize) -> Vec<f64> {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let mut msg = BiWiMessage::new();
        msg.set_field(1, json_to_biwi(&s.payload));
        let start = Instant::now();
        let _ = msg.to_buffer();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples
}

fn benchmark_decode(s: &Scenario, iterations: usize) -> Vec<f64> {
    let mut msg = BiWiMessage::new();
    msg.set_field(1, json_to_biwi(&s.payload));
    let buffer = msg.to_buffer();
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = BiWiMessage::from_buffer(&buffer).unwrap();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples
}

fn benchmark_round_trip(stream: &mut TcpStream, s: &Scenario, iterations: usize) -> (f64, f64, f64, f64, f64) {
    let mut msg = BiWiMessage::new();
    msg.set_field(1, json_to_biwi(&s.payload));
    let payload = msg.to_buffer();

    let mut samples = Vec::with_capacity(iterations);
    let mut resp = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        let len = payload.len() as u32;
        stream.write_all(&len.to_be_bytes()).unwrap();
        stream.write_all(&payload).unwrap();

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).unwrap();
        let resp_len = u32::from_be_bytes(len_buf) as usize;
        resp.resize(resp_len, 0);
        stream.read_exact(&mut resp).unwrap();
        let _ = BiWiMessage::from_buffer(&resp).unwrap();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    calc_stats(samples)
}

fn throughput_test(stream: &mut TcpStream, messages: usize) -> ThroughputResult {
    let mut msg = BiWiMessage::new();
    msg.set_field(1, json_to_biwi(&scenarios()[0].payload));

    let start = Instant::now();
    for i in 0..messages {
        // slightly vary data to avoid caching artifacts
        {
            if let Some(BiWiValue::Object(map)) = msg.get_field_mut(1) {
                if let Some(BiWiValue::String(ref mut s)) = map.get_mut("playerId") {
                    *s = format!("player_{}", i % 1000);
                }
            }
        }
        let buf = msg.to_vec();
        let len_bytes = (buf.len() as u32).to_be_bytes();
        stream.write_all(&len_bytes).unwrap();
        stream.write_all(&buf).unwrap();
    }

    let mut received = 0usize;
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
    ThroughputResult { label: "BiWi", throughput, total_time_ms: elapsed }
}

fn message_size(s: &Scenario) -> usize {
    let mut msg = BiWiMessage::new();
    msg.set_field(1, json_to_biwi(&s.payload));
    msg.size()
}

fn json_to_biwi(value: &serde_json::Value) -> BiWiValue {
    match value {
        serde_json::Value::Null => BiWiValue::Null,
        serde_json::Value::Bool(b) => BiWiValue::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    BiWiValue::Int32(i as i32)
                } else {
                    BiWiValue::Int64(i)
                }
            } else if let Some(f) = n.as_f64() {
                BiWiValue::number(f)
            } else {
                BiWiValue::Null
            }
        }
        serde_json::Value::String(s) => BiWiValue::from(s.as_str()),
        serde_json::Value::Array(arr) => {
            BiWiValue::Array(arr.iter().map(json_to_biwi).collect())
        }
        serde_json::Value::Object(map) => {
            let mut obj = std::collections::HashMap::new();
            for (k, v) in map {
                obj.insert(k.clone(), json_to_biwi(v));
            }
            BiWiValue::Object(obj)
        }
    }
}
