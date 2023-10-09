fn main() {
    prost_build::Config::new()
        .out_dir("src")
        .compile_protos(&["gtfs_realtime.proto"])
        .unwrap();
}
 
