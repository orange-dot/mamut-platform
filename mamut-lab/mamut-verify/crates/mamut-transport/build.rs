//! Build script for mamut-transport.
//!
//! This script compiles the Protocol Buffer definitions into Rust code
//! using tonic-build.

use std::io::Result;

fn main() -> Result<()> {
    // Configure tonic-build
    let mut config = tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .build_transport(true);

    // Add custom attributes for generated types
    config = config
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]");

    // Compile the proto files
    // Note: We compile both files together since mamut.proto imports ovc.proto
    config.compile_protos(
        &["../../proto/mamut.proto", "../../proto/ovc.proto"],
        &["../../proto"],
    )?;

    // Re-run if proto files change
    println!("cargo::rerun-if-changed=../../proto/mamut.proto");
    println!("cargo::rerun-if-changed=../../proto/ovc.proto");

    Ok(())
}
