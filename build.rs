//! Build script for gRPC code generation.
//!
//! Proto code is pre-generated and committed to `src/transport/proto/`.
//! This build script only regenerates if the generated file is missing.
//! Use `make proto` to manually regenerate after updating proto files.

// Build scripts should panic on failure - that's how they signal build errors
#![allow(clippy::expect_used)]

fn main() {
    #[cfg(feature = "grpc")]
    {
        // Path to the proto file (bundled with the SDK via git submodule)
        let proto_file = "proto/inferadb/authorization/v1/authorization.proto";
        let proto_dir = "proto";
        let generated_file = "src/transport/proto/inferadb.authorization.v1.rs";

        // Skip generation if the generated file already exists (it's committed to the repo)
        // This prevents build.rs from modifying src/ during cargo publish
        if std::path::Path::new(generated_file).exists() {
            println!("cargo:rerun-if-changed={generated_file}");
            return;
        }

        // Check if proto file exists
        if !std::path::Path::new(proto_file).exists() {
            println!(
                "cargo:warning=Proto file not found at {proto_file}, skipping code generation"
            );
            return;
        }

        // Tell cargo to rerun if the proto file changes
        println!("cargo:rerun-if-changed={proto_file}");

        // Configure tonic-prost-build (tonic 0.14+ split prost codegen into separate crate)
        tonic_prost_build::configure()
            .build_server(false) // We only need the client
            .build_client(true)
            .out_dir("src/transport/proto")
            .compile_protos(&[proto_file], &[proto_dir])
            .expect("Failed to compile proto files");
    }
}
