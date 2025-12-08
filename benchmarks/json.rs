use crate::benchmarks::{calc_stats, scenarios, Scenario, StatResult, ThroughputResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Instant;

#[derive(Serialize, Deserialize)]
struct Envelope {
    req_id: u32,
    value: Value,
}

pub fn run_json_benchmark() -> (Vec<StatResult>, ThroughputResult) {
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

        let size = serde_json::to_vec(&scenario.payload).unwrap().len() + 1; // newline framing

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
    let listener = TcpListener::bind("127.0.0.1:4011").expect("bind json");
    let _server = thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                s.set_nodelay(true).ok();
                thread::spawn(move || handle_client(&mut s));
            }
        }
    });
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut results = Vec::new();
    let scens = scenarios();
    for scenario in &scens {
        let mut stream = TcpStream::connect("127.0.0.1:4011").expect("connect json");
        stream.set_nodelay(true).expect("set_nodelay");
        let (avg, min, max, p95, p99) = bench_round_trip(&mut stream, scenario, 200);
        let size = serde_json::to_vec(&scenario.payload).unwrap().len() + 1;
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

    let mut stream = TcpStream::connect("127.0.0.1:4011").expect("connect json throughput");
    stream.set_nodelay(true).expect("set_nodelay");
    let throughput = throughput_test(&mut stream, 1_000);
    (results, throughput)
}

fn handle_client(stream: &mut TcpStream) {
    let mut buffer = String::new();
    let mut read_buf = [0u8; 4096];
    loop {
        match stream.read(&mut read_buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buffer.push_str(&String::from_utf8_lossy(&read_buf[..n]));
                while let Some(idx) = buffer.find('\n') {
                    let line = buffer[..idx].to_string();
                    buffer = buffer[idx + 1..].to_string();
                    if line.is_empty() {
                        continue;
                    }
                    if let Ok(env) = serde_json::from_str::<Envelope>(&line) {
                        let resp = Envelope { req_id: env.req_id, value: env.value };
                        let out = serde_json::to_string(&resp).unwrap() + "\n";
                        if stream.write_all(out.as_bytes()).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    }
}

fn bench_encode(s: &Scenario, iterations: usize) -> Vec<f64> {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = serde_json::to_vec(&s.payload).unwrap();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples
}

fn bench_decode(s: &Scenario, iterations: usize) -> Vec<f64> {
    let buf = serde_json::to_vec(&s.payload).unwrap();
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let _: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples
}

fn bench_round_trip(stream: &mut TcpStream, s: &Scenario, iterations: usize) -> (f64, f64, f64, f64, f64) {
    let mut samples = Vec::with_capacity(iterations);
    let mut buffer = String::new();
    let mut read_buf = [0u8; 4096];

    for i in 0..iterations {
        buffer.clear();
        let env = Envelope { req_id: i as u32, value: s.payload.clone() };
        let line = serde_json::to_string(&env).unwrap() + "\n";
        let start = Instant::now();
        stream.write_all(line.as_bytes()).unwrap();

        let mut acc = String::new();
        loop {
            let n = stream.read(&mut read_buf).unwrap();
            acc.push_str(&String::from_utf8_lossy(&read_buf[..n]));
            if let Some(idx) = acc.find('\n') {
                let resp_line = acc[..idx].to_string();
                let _ = serde_json::from_str::<Envelope>(&resp_line).unwrap();
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
                break;
            }
        }
    }

    calc_stats(samples)
}

fn throughput_test(stream: &mut TcpStream, messages: usize) -> ThroughputResult {
    let mut buffer = String::new();
    let mut read_buf = [0u8; 4096];
    let payload = scenarios()[0].payload.clone();

    let start = Instant::now();
    for i in 0..messages {
        let env = Envelope { req_id: i as u32, value: payload.clone() };
        let line = serde_json::to_string(&env).unwrap() + "\n";
        stream.write_all(line.as_bytes()).unwrap();
    }

    let mut received = 0usize;
    while received < messages {
        let n = stream.read(&mut read_buf).unwrap();
        buffer.push_str(&String::from_utf8_lossy(&read_buf[..n]));
        while let Some(idx) = buffer.find('\n') {
            let line = buffer[..idx].to_string();
            buffer = buffer[idx + 1..].to_string();
            if !line.is_empty() {
                let _ = serde_json::from_str::<Envelope>(&line).unwrap();
                received += 1;
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let throughput = messages as f64 / (elapsed / 1000.0);
    ThroughputResult { label: "JSON", throughput, total_time_ms: elapsed }
}
