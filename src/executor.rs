use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::docker::{self, ContainerConfig};

pub struct DockerExecutor {
    docker: docker::DockerClient,
}

impl DockerExecutor {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            docker: docker::DockerClient::new().await?,
        })
    }
    
    pub async fn execute(&self, 
        execution_id: Uuid,
        code: &str,
        language: &str,
        args: Vec<String>,
        environment: HashMap<String, String>,
        timeout_seconds: Option<i32>,
    ) -> Result<docker::ExecutionResult> {
        let config = ContainerConfig {
            image: self.get_image_for_language(language),
            command: self.get_command_for_language(language, &args),
            environment,
            working_dir: "/workspace".to_string(),
            memory_limit: Some(512 * 1024 * 1024), // 512MB
            cpu_limit: Some(1.0),
            timeout_seconds: timeout_seconds.map(|t| t as u64),
        };
        
        // Create temporary file for code
        let temp_dir = tempfile::tempdir()?;
        let code_file = temp_dir.path().join(self.get_filename_for_language(language));
        std::fs::write(&code_file, code)?;
        
        // Execute in container
        let result = self.docker.run_container(
            &format!("execution-{}", execution_id),
            config,
            Some(temp_dir.path()),
        ).await?;
        
        Ok(result)
    }
    
    fn get_image_for_language(&self, language: &str) -> String {
        match language {
            "python" => "python:3.11-slim",
            "javascript" | "typescript" => "node:20-slim",
            "rust" => "rust:1.75-slim",
            "go" => "golang:1.21-alpine",
            "java" => "openjdk:17-slim",
            "ruby" => "ruby:3.2-slim",
            "php" => "php:8.2-cli",
            _ => "ubuntu:22.04",
        }.to_string()
    }
    
    fn get_command_for_language(&self, language: &str, args: &[String]) -> Vec<String> {
        let mut cmd = match language {
            "python" => vec!["python", "main.py"],
            "javascript" => vec!["node", "main.js"],
            "typescript" => vec!["npx", "tsx", "main.ts"],
            "rust" => vec!["cargo", "run"],
            "go" => vec!["go", "run", "main.go"],
            "java" => vec!["java", "Main.java"],
            "ruby" => vec!["ruby", "main.rb"],
            "php" => vec!["php", "main.php"],
            "shell" => vec!["sh", "main.sh"],
            _ => vec!["sh", "-c"],
        }.into_iter().map(String::from).collect::<Vec<_>>();
        
        cmd.extend(args.iter().cloned());
        cmd
    }
    
    fn get_filename_for_language(&self, language: &str) -> &str {
        match language {
            "python" => "main.py",
            "javascript" => "main.js",
            "typescript" => "main.ts",
            "rust" => "main.rs",
            "go" => "main.go",
            "java" => "Main.java",
            "ruby" => "main.rb",
            "php" => "main.php",
            "shell" => "main.sh",
            _ => "main.txt",
        }
    }
}