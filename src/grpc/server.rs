use super::proto::syla::execution::v1 as proto;
use super::proto::syla::common::v1::{HealthCheckRequest, HealthCheckResponse, HealthStatus, PageRequest, PageResponse};
use super::IntoStatus;
use crate::executor::DockerExecutor;
use crate::models::{Execution, ExecutionStatus as DbExecutionStatus};
use crate::queue::RedisQueue;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct ExecutionServiceImpl {
    queue: Arc<RedisQueue>,
    executor: Arc<DockerExecutor>,
    // In-memory storage for now, will be replaced with database
    executions: Arc<RwLock<std::collections::HashMap<String, Execution>>>,
}

impl ExecutionServiceImpl {
    pub fn new(queue: Arc<RedisQueue>, executor: Arc<DockerExecutor>) -> Self {
        Self {
            queue,
            executor,
            executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    // Convert database execution to proto execution
    fn to_proto_execution(&self, db_exec: &Execution) -> proto::Execution {
        proto::Execution {
            id: db_exec.id.to_string(),
            user_id: db_exec.user_id.clone(),
            workspace_id: db_exec.workspace_id.clone().unwrap_or_default(),
            request: Some(proto::ExecutionRequest {
                code: db_exec.code.clone(),
                language: self.language_to_proto(&db_exec.language) as i32,
                args: db_exec.args.clone().unwrap_or_default(),
                environment: db_exec.environment.clone().unwrap_or_default(),
                resources: Some(proto::ResourceRequirements {
                    memory_mb: 512, // Default for now
                    cpu_cores: 1.0,
                    disk_mb: 100,
                    enable_network: false,
                    enable_gpu: false,
                }),
                timeout: Some(prost_types::Duration {
                    seconds: db_exec.timeout_seconds.unwrap_or(30) as i64,
                    nanos: 0,
                }),
                files: vec![],
                mode: proto::ExecutionMode::Sandbox as i32,
                metadata: std::collections::HashMap::new(),
            }),
            status: self.status_to_proto(&db_exec.status) as i32,
            result: db_exec.exit_code.map(|code| proto::ExecutionResult {
                exit_code: code,
                stdout: db_exec.stdout.clone().unwrap_or_default(),
                stderr: db_exec.stderr.clone().unwrap_or_default(),
                files: vec![],
                outputs: std::collections::HashMap::new(),
                error: if code != 0 {
                    Some(proto::ExecutionError {
                        code: "EXECUTION_FAILED".to_string(),
                        message: format!("Process exited with code {}", code),
                        details: db_exec.stderr.clone().unwrap_or_default(),
                        stack_trace: String::new(),
                    })
                } else {
                    None
                },
            }),
            created_at: Some(prost_types::Timestamp {
                seconds: db_exec.created_at.timestamp(),
                nanos: 0,
            }),
            started_at: db_exec.started_at.map(|t| prost_types::Timestamp {
                seconds: t.timestamp(),
                nanos: 0,
            }),
            completed_at: db_exec.completed_at.map(|t| prost_types::Timestamp {
                seconds: t.timestamp(),
                nanos: 0,
            }),
            worker_id: String::new(),
            metrics: None,
        }
    }
    
    fn language_to_proto(&self, lang: &str) -> proto::Language {
        match lang {
            "python" => proto::Language::Python,
            "javascript" => proto::Language::Javascript,
            "typescript" => proto::Language::Typescript,
            "rust" => proto::Language::Rust,
            "go" => proto::Language::Go,
            "java" => proto::Language::Java,
            "cpp" => proto::Language::Cpp,
            "csharp" => proto::Language::Csharp,
            "ruby" => proto::Language::Ruby,
            "php" => proto::Language::Php,
            "shell" => proto::Language::Shell,
            _ => proto::Language::Unspecified,
        }
    }
    
    fn status_to_proto(&self, status: &DbExecutionStatus) -> proto::ExecutionStatus {
        match status {
            DbExecutionStatus::Pending => proto::ExecutionStatus::Pending,
            DbExecutionStatus::Running => proto::ExecutionStatus::Running,
            DbExecutionStatus::Completed => proto::ExecutionStatus::Completed,
            DbExecutionStatus::Failed => proto::ExecutionStatus::Failed,
            DbExecutionStatus::Cancelled => proto::ExecutionStatus::Cancelled,
        }
    }
    
    fn proto_to_language(&self, lang: proto::Language) -> String {
        match lang {
            proto::Language::Python => "python",
            proto::Language::Javascript => "javascript",
            proto::Language::Typescript => "typescript",
            proto::Language::Rust => "rust",
            proto::Language::Go => "go",
            proto::Language::Java => "java",
            proto::Language::Cpp => "cpp",
            proto::Language::Csharp => "csharp",
            proto::Language::Ruby => "ruby",
            proto::Language::Php => "php",
            proto::Language::Shell => "shell",
            _ => "unknown",
        }.to_string()
    }
}

#[tonic::async_trait]
impl proto::execution_service_server::ExecutionService for ExecutionServiceImpl {
    async fn submit_execution(
        &self,
        request: Request<proto::SubmitExecutionRequest>,
    ) -> Result<Response<proto::SubmitExecutionResponse>, Status> {
        let req = request.into_inner();
        let exec_req = req.request.ok_or_else(|| Status::invalid_argument("Missing execution request"))?;
        
        // Create execution record
        let execution_id = Uuid::new_v4();
        let context = req.context.unwrap_or_default();
        
        let execution = Execution {
            id: execution_id,
            user_id: context.user_id,
            workspace_id: Some(context.workspace_id),
            code: exec_req.code,
            language: self.proto_to_language(proto::Language::try_from(exec_req.language).unwrap_or(proto::Language::Unspecified)),
            args: Some(exec_req.args),
            environment: Some(exec_req.environment),
            timeout_seconds: exec_req.timeout.map(|d| d.seconds as i32),
            status: DbExecutionStatus::Pending,
            exit_code: None,
            stdout: None,
            stderr: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        };
        
        // Store execution
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id.to_string(), execution.clone());
        }
        
        // Queue for processing
        self.queue.push_job(execution_id).await
            .map_err(|e| Status::internal(format!("Failed to queue job: {}", e)))?;
        
        // If sync execution requested, wait for completion
        let result = if !req.r#async {
            // TODO: Implement sync execution with timeout
            None
        } else {
            None
        };
        
        Ok(Response::new(proto::SubmitExecutionResponse {
            execution_id: execution_id.to_string(),
            status: proto::ExecutionStatus::Pending as i32,
            result,
        }))
    }
    
    async fn get_execution(
        &self,
        request: Request<proto::GetExecutionRequest>,
    ) -> Result<Response<proto::GetExecutionResponse>, Status> {
        let req = request.into_inner();
        
        let executions = self.executions.read().await;
        let execution = executions.get(&req.execution_id)
            .ok_or_else(|| Status::not_found("Execution not found"))?;
        
        Ok(Response::new(proto::GetExecutionResponse {
            execution: Some(self.to_proto_execution(execution)),
        }))
    }
    
    async fn stream_execution(
        &self,
        request: Request<proto::StreamExecutionRequest>,
    ) -> Result<Response<Self::StreamExecutionStream>, Status> {
        // TODO: Implement streaming
        Err(Status::unimplemented("Streaming not yet implemented"))
    }
    
    type StreamExecutionStream = tonic::codec::Streaming<proto::ExecutionEvent>;
    
    async fn cancel_execution(
        &self,
        request: Request<proto::CancelExecutionRequest>,
    ) -> Result<Response<proto::CancelExecutionResponse>, Status> {
        let req = request.into_inner();
        
        // TODO: Implement cancellation
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(&req.execution_id) {
            execution.status = DbExecutionStatus::Cancelled;
            execution.completed_at = Some(chrono::Utc::now());
            
            Ok(Response::new(proto::CancelExecutionResponse {
                success: true,
                final_status: proto::ExecutionStatus::Cancelled as i32,
            }))
        } else {
            Err(Status::not_found("Execution not found"))
        }
    }
    
    async fn list_executions(
        &self,
        request: Request<proto::ListExecutionsRequest>,
    ) -> Result<Response<proto::ListExecutionsResponse>, Status> {
        let req = request.into_inner();
        
        let executions = self.executions.read().await;
        let mut results = Vec::new();
        
        for (_, execution) in executions.iter() {
            // Filter by user_id if provided
            if !req.user_id.is_empty() && execution.user_id != req.user_id {
                continue;
            }
            
            // Filter by workspace_id if provided
            if !req.workspace_id.is_empty() {
                if let Some(ws_id) = &execution.workspace_id {
                    if ws_id != &req.workspace_id {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            
            results.push(self.to_proto_execution(execution));
        }
        
        // Apply pagination
        let page = req.page.unwrap_or_default();
        let page_size = page.size.max(10).min(100) as usize;
        let page_num = page.number.max(1) as usize;
        let start = (page_num - 1) * page_size;
        let end = (start + page_size).min(results.len());
        
        let total = results.len() as u32;
        let paginated_results = results[start..end].to_vec();
        
        Ok(Response::new(proto::ListExecutionsResponse {
            executions: paginated_results,
            page: Some(PageResponse {
                total,
                size: page_size as u32,
                number: page_num as u32,
                total_pages: (total as f32 / page_size as f32).ceil() as u32,
            }),
        }))
    }
    
    async fn get_execution_metrics(
        &self,
        request: Request<proto::GetExecutionMetricsRequest>,
    ) -> Result<Response<proto::GetExecutionMetricsResponse>, Status> {
        // TODO: Implement metrics collection
        Err(Status::unimplemented("Metrics not yet implemented"))
    }
    
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: HealthStatus::Healthy as i32,
            message: "Execution service is healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: Some(prost_types::Duration {
                seconds: 0, // TODO: Track uptime
                nanos: 0,
            }),
            metadata: std::collections::HashMap::new(),
        }))
    }
}