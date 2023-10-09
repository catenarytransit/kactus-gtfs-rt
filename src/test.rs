extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["gtfs-realtime.proto"], &["src/"]).unwrap();
} 
