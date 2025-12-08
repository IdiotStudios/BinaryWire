mod benchmarks;

use benchmarks::{StatResult, ThroughputResult};
use std::fs::File;
use std::io::Write;

const RESULTS_CSV: &str = "benchmark_results.csv";

fn main() {
    println!("=== BiWi Benchmarks ===");
    println!("May take a while...");
    let (biwi_stats, biwi_tp) = benchmarks::biwi::run_biwi_benchmark();
    print_stats("BiWi", &biwi_stats);
    print_throughput(&biwi_tp);

    println!("\n=== JSON Benchmarks ===");
    println!("May take a while...");
    let (json_stats, json_tp) = benchmarks::json::run_json_benchmark();
    print_stats("JSON", &json_stats);
    print_throughput(&json_tp);

    println!("\n=== Protobuf Benchmarks ===");
    println!("May take a while...");
    let (proto_stats, proto_tp) = benchmarks::protobuf::run_protobuf_benchmark();
    print_stats("Protobuf", &proto_stats);
    print_throughput(&proto_tp);

    if let Err(err) = write_csv(&biwi_stats, &biwi_tp, &json_stats, &json_tp, &proto_stats, &proto_tp) {
        eprintln!("Failed to write {}: {}", RESULTS_CSV, err);
    } else {
        println!("\nCSV results written to {}", RESULTS_CSV);
    }
}

fn print_stats(label: &str, stats: &[StatResult]) {
    println!("Protocol: {}", label);
    println!("{:35} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}", "Scenario", "avg", "min", "max", "p95", "p99", "bytes");
    for s in stats {
        println!(
            "{:35} {:8.4} {:8.4} {:8.4} {:8.4} {:8.4} {:8}",
            s.scenario,
            s.avg_ms,
            s.min_ms,
            s.max_ms,
            s.p95_ms,
            s.p99_ms,
            s.size_bytes
        );
    }
}

fn print_throughput(t: &ThroughputResult) {
    println!(
        "Throughput {}: {:.2} msg/s over {:.2} ms",
        t.label, t.throughput, t.total_time_ms
    );
}

fn write_csv(
    biwi_stats: &[StatResult],
    biwi_tp: &ThroughputResult,
    json_stats: &[StatResult],
    json_tp: &ThroughputResult,
    proto_stats: &[StatResult],
    proto_tp: &ThroughputResult,
) -> std::io::Result<()> {
    let mut file = File::create(RESULTS_CSV)?;
    writeln!(
        file,
        "protocol,scenario,avg_ms,min_ms,max_ms,p95_ms,p99_ms,size_bytes,throughput_msg_s,throughput_total_ms"
    )?;

    write_stat_rows(&mut file, "BiWi", biwi_stats)?;
    write_throughput_row(&mut file, "BiWi", biwi_tp)?;

    write_stat_rows(&mut file, "JSON", json_stats)?;
    write_throughput_row(&mut file, "JSON", json_tp)?;

    write_stat_rows(&mut file, "Protobuf", proto_stats)?;
    write_throughput_row(&mut file, "Protobuf", proto_tp)?;

    Ok(())
}

fn write_stat_rows(file: &mut File, protocol: &str, stats: &[StatResult]) -> std::io::Result<()> {
    for s in stats {
        writeln!(
            file,
            "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},,",
            protocol,
            s.scenario,
            s.avg_ms,
            s.min_ms,
            s.max_ms,
            s.p95_ms,
            s.p99_ms,
            s.size_bytes
        )?;
    }
    Ok(())
}

fn write_throughput_row(file: &mut File, protocol: &str, t: &ThroughputResult) -> std::io::Result<()> {
    writeln!(
        file,
        "{},{},{},{},{},{},{},{},{:.2},{:.2}",
        protocol,
        "THROUGHPUT",
        "",
        "",
        "",
        "",
        "",
        "",
        t.throughput,
        t.total_time_ms
    )
}
