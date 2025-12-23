//! Build script for gRPC code generation.

fn main() {
    #[cfg(feature = "grpc")]
    {
        // Path to the proto file (bundled with the SDK)
        let proto_file = "proto/inferadb.proto";
        let proto_dir = "proto";

        // Check if proto file exists
        if !std::path::Path::new(proto_file).exists() {
            // This shouldn't happen in a properly distributed crate
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
