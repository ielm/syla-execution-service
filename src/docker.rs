use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

pub struct DockerExecutor;

impl DockerExecutor {
    pub fn new() -> Result<Self> {
        // Verify Docker is available
        Command::new("docker")
            .arg("--version")
            .output()
            .context("Docker not found. Please install Docker.")?;
        
        Ok(Self)
    }
    
    pub async fn execute(
        &self,
        code: &str,
        language: &str,
        timeout_seconds: u64,
    ) -> Result<ExecutionResult> {
        // For MVP, create a temporary file
        let temp_dir = tempfile::tempdir()?;
        let file_extension = match language {
            "python" => "py",
            "javascript" => "js",
            "go" => "go",
            _ => "txt",
        };
        
        let file_path = temp_dir.path().join(format!("main.{}", file_extension));
        std::fs::write(&file_path, code)?;
        
        // Get Docker image
        let image = match language {
            "python" => "python:3.11-slim",
            "javascript" => "node:20-slim",
            "go" => "golang:1.21-alpine",
            _ => "ubuntu:22.04",
        };
        
        // Build Docker command
        let container_name = format!("syla-exec-{}", Uuid::new_v4());
        let mut cmd = TokioCommand::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("--name").arg(&container_name)
            .arg("-v").arg(format!("{}:/workspace:ro", temp_dir.path().display()))
            .arg("-w").arg("/workspace")
            .arg("--memory").arg("512m")
            .arg("--cpus").arg("1")
            .arg(image);
            
        // Add language-specific command
        match language {
            "python" => cmd.arg("python").arg("main.py"),
            "javascript" => cmd.arg("node").arg("main.js"),
            "go" => cmd.arg("go").arg("run").arg("main.go"),
            _ => &mut cmd,
        };
        
        // Execute with timeout
        let start = std::time::Instant::now();
        let output = tokio::time::timeout(
            Duration::from_secs(timeout_seconds),
            cmd.output()
        ).await;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        match output {
            Ok(Ok(output)) => {
                Ok(ExecutionResult {
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    duration_ms,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                // Timeout - try to kill container
                let _ = Command::new("docker")
                    .args(&["kill", &container_name])
                    .output();
                    
                Ok(ExecutionResult {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: "Execution timed out".to_string(),
                    duration_ms,
                    timed_out: true,
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}