use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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