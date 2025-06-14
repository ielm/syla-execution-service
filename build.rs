use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Rerun if proto files change
    println!("cargo:rerun-if-changed=proto/execution.proto");
    println!("cargo:rerun-if-changed=build.rs");
    
    // Get OUT_DIR from cargo
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    
    // Configure tonic-build
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        // Only add serde derives to types that don't contain google.protobuf types
        .type_attribute("syla.execution.v1.Language", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.execution.v1.ExecutionMode", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.execution.v1.ExecutionStatus", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.execution.v1.OutputType", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.execution.v1.WorkerStatus", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.common.v1.HealthStatus", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["proto/execution.proto"],
            &["proto"], // Include directory with symlinks
        )?;
    
    println!("cargo:warning=Proto compilation completed successfully");
    
    Ok(())
}