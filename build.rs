fn main() {
    let proto_path = "benchmarks/messages.proto";
    println!("cargo:rerun-if-changed={proto_path}");

    prost_build::Config::new()
        .btree_map(["."])
        .compile_protos(&[proto_path], &["benchmarks"]).expect("protos");
}
