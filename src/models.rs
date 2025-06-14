use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExecutionRequest {
    pub code: String,
    pub language: String,
    pub timeout_seconds: Option<u64>,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionJob {
    pub id: Uuid,
    pub status: JobStatus,
    pub request: CreateExecutionRequest,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<ExecutionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

// Database models for persistence
#[derive(Debug, Clone)]
pub struct Execution {
    pub id: Uuid,
    pub user_id: String,
    pub workspace_id: Option<String>,
    pub code: String,
    pub language: String,
    pub args: Option<Vec<String>>,
    pub environment: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<i32>,
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ExecutionJob {
    pub fn new(request: CreateExecutionRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: JobStatus::Queued,
            request,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
        }
    }
}