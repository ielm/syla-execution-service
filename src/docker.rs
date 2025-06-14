use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

pub struct DockerClient {
    // Future: connection pool, etc
}

pub struct ContainerConfig {
    pub image: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_dir: String,
    pub memory_limit: Option<u64>,
    pub cpu_limit: Option<f64>,
    pub timeout_seconds: Option<u64>,
}

// Legacy DockerExecutor for backward compatibility
pub struct DockerExecutor {
    client: DockerClient,
}

impl DockerExecutor {
    pub fn new() -> Result<Self> {
        // Verify Docker is available
        Command::new("docker")
            .arg("--version")
            .output()
            .context("Docker not found. Please install Docker.")?;
        
        Ok(Self {
            client: DockerClient {},
        })
    }
    
    pub async fn execute(
        &self,
        code: &str,
        language: &str,
        timeout_seconds: u64,
    ) -> Result<ExecutionResult> {
        let temp_dir = tempfile::tempdir()?;
        let file_extension = match language {
            "python" => "py",
            "javascript" => "js",
            "go" => "go",
            _ => "txt",
        };
        
        let file_path = temp_dir.path().join(format!("main.{}", file_extension));
        std::fs::write(&file_path, code)?;
        
        let config = ContainerConfig {
            image: match language {
                "python" => "python:3.11-slim",
                "javascript" => "node:20-slim",
                "go" => "golang:1.21-alpine",
                _ => "ubuntu:22.04",
            }.to_string(),
            command: match language {
                "python" => vec!["python".to_string(), "main.py".to_string()],
                "javascript" => vec!["node".to_string(), "main.js".to_string()],
                "go" => vec!["go".to_string(), "run".to_string(), "main.go".to_string()],
                _ => vec![],
            },
            environment: HashMap::new(),
            working_dir: "/workspace".to_string(),
            memory_limit: Some(512 * 1024 * 1024),
            cpu_limit: Some(1.0),
            timeout_seconds: Some(timeout_seconds),
        };
        
        self.client.run_container(
            &format!("syla-exec-{}", Uuid::new_v4()),
            config,
            Some(temp_dir.path()),
        ).await
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

impl DockerClient {
    pub async fn new() -> Result<Self> {
        // Verify Docker is available
        Command::new("docker")
            .arg("--version")
            .output()
            .context("Docker not found. Please install Docker.")?;
        
        Ok(Self {})
    }
    
    pub async fn run_container(
        &self,
        name: &str,
        config: ContainerConfig,
        mount_path: Option<&Path>,
    ) -> Result<ExecutionResult> {
        let mut cmd = TokioCommand::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("--name").arg(name);
            
        // Add volume mount if provided
        if let Some(path) = mount_path {
            cmd.arg("-v").arg(format!("{}:{}:ro", path.display(), config.working_dir));
        }
        
        // Set working directory
        cmd.arg("-w").arg(&config.working_dir);
        
        // Resource limits
        if let Some(memory) = config.memory_limit {
            cmd.arg("--memory").arg(format!("{}", memory));
        }
        if let Some(cpus) = config.cpu_limit {
            cmd.arg("--cpus").arg(format!("{}", cpus));
        }
        
        // Environment variables
        for (key, value) in &config.environment {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }
        
        // Image and command
        cmd.arg(&config.image);
        cmd.args(&config.command);
        
        // Execute with timeout
        let start = std::time::Instant::now();
        let timeout = config.timeout_seconds.unwrap_or(30);
        let output = tokio::time::timeout(
            Duration::from_secs(timeout),
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
                    .args(&["kill", name])
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