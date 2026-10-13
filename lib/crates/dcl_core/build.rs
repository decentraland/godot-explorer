use std::{env, path::Path};

// The protocol tree is installed by `cargo run -- install` next to the engine's own protos.
const PROTO_FILES_BASE_DIR: &str = "../dcl_engine/src/dcl/components/proto/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut protoc_path = env::current_dir()?.join("../../../.bin/protoc/bin/protoc");
    if env::consts::OS == "windows" {
        protoc_path.set_extension("exe");
    }
    let protoc_path = protoc_path
        .canonicalize()
        .expect("Failed to canonicalize protoc path");
    env::set_var("PROTOC", protoc_path);

    let proto_files = [
        format!("{PROTO_FILES_BASE_DIR}decentraland/social_service/errors.proto"),
        format!("{PROTO_FILES_BASE_DIR}decentraland/social_service/v2/social_service_v2.proto"),
    ];

    let mut prost_config = prost_build::Config::new();
    prost_config.type_attribute(".", "#[derive(serde::Serialize)]");
    prost_config.service_generator(Box::new(dcl_rpc::codegen::RPCServiceGenerator::new()));
    prost_config.compile_protos(&proto_files, &[Path::new(PROTO_FILES_BASE_DIR)])?;

    for source in &proto_files {
        println!("cargo:rerun-if-changed={source}");
    }
    Ok(())
}
