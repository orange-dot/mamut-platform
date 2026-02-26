use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest_dir.join("../../../proto").canonicalize()?;
    let mamut_dir = proto_root.join("mamut");

    let proto_files = [
        "common.proto",
        "conveyor.proto",
        "diverter.proto",
        "identification.proto",
        "tracking.proto",
        "wms.proto",
        "alarms.proto",
        "telemetry.proto",
    ]
    .into_iter()
    .map(|name| mamut_dir.join(name))
    .collect::<Vec<_>>();

    for file in &proto_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    // Build scripts run in a controlled process, and setting PROTOC here is
    // required so tonic-build uses the vendored compiler binary.
    unsafe {
        env::set_var("PROTOC", protoc);
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&proto_files, &[proto_root])?;

    Ok(())
}
