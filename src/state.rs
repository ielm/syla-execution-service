use crate::error::ServiceError;
use crate::models::{CreateExecutionRequest, ExecutionJob};
use anyhow::Result;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct ServiceState {
    pub redis: Arc<Mutex<ConnectionManager>>,
    pub docker_executor: Arc<crate::docker::DockerExecutor>,
}

impl ServiceState {
    pub async fn create_execution(
        &self,
        request: CreateExecutionRequest,
    ) -> Result<ExecutionJob, ServiceError> {
        let job = ExecutionJob::new(request);
        
        // Store job in Redis
        let mut redis = self.redis.lock().await;
        let job_key = format!("job:{}", job.id);
        let job_json = serde_json::to_string(&job)?;
        
        redis::cmd("SET")
            .arg(&job_key)
            .arg(&job_json)
            .query_async::<_, ()>(&mut *redis)
            .await?;
        
        // Add to queue
        redis::cmd("RPUSH")
            .arg("execution_queue")
            .arg(job.id.to_string())
            .query_async::<_, ()>(&mut *redis)
            .await?;
        
        Ok(job)
    }
    
    pub async fn get_execution(&self, id: Uuid) -> Result<ExecutionJob, ServiceError> {
        let mut redis = self.redis.lock().await;
        let job_key = format!("job:{}", id);
        
        let job_json: Option<String> = redis::cmd("GET")
            .arg(&job_key)
            .query_async(&mut *redis)
            .await?;
        
        match job_json {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Err(ServiceError::NotFound),
        }
    }
}