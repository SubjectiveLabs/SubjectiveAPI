use prost_build::compile_protos;

fn main() {
    compile_protos(&["src/proto/gtfs-realtime.proto"], &["src/"]).unwrap();
}
