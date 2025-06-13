use crate::models::{ExecutionJob, ExecutionResult, JobStatus};
use crate::state::ServiceState;
use std::sync::Arc;
use tracing::{error, info};

pub async fn run_worker(state: Arc<ServiceState>) {
    info!("Starting execution worker");
    
    loop {
        // Get job from queue
        let job_id = {
            let mut redis = state.redis.lock().await;
            let result: Result<Option<String>, _> = redis::cmd("LPOP")
                .arg("execution_queue")
                .query_async(&mut *redis)
                .await;
                
            match result {
                Ok(Some(id)) => id,
                Ok(None) => {
                    // No jobs, wait a bit
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => {
                    error!("Redis error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            }
        };
        
        // Parse job ID
        let job_id = match job_id.parse::<uuid::Uuid>() {
            Ok(id) => id,
            Err(e) => {
                error!("Invalid job ID: {}", e);
                continue;
            }
        };
        
        // Process job
        if let Err(e) = process_job(&state, job_id).await {
            error!("Error processing job {}: {}", job_id, e);
        }
    }
}

async fn process_job(state: &ServiceState, job_id: uuid::Uuid) -> anyhow::Result<()> {
    info!("Processing job {}", job_id);
    
    // Get job details
    let mut job = state.get_execution(job_id).await?;
    
    // Update status to running
    job.status = JobStatus::Running;
    job.started_at = Some(chrono::Utc::now());
    update_job(state, &job).await?;
    
    // Execute
    let result = state.docker_executor
        .execute(
            &job.request.code,
            &job.request.language,
            job.request.timeout_seconds.unwrap_or(30),
        )
        .await;
    
    // Update job with result
    match result {
        Ok(exec_result) => {
            job.status = if exec_result.timed_out {
                JobStatus::Timeout
            } else if exec_result.exit_code == 0 {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            };
            
            job.result = Some(ExecutionResult {
                exit_code: exec_result.exit_code,
                stdout: exec_result.stdout,
                stderr: exec_result.stderr,
                duration_ms: exec_result.duration_ms,
            });
        }
        Err(e) => {
            job.status = JobStatus::Failed;
            job.result = Some(ExecutionResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Execution error: {}", e),
                duration_ms: 0,
            });
        }
    }
    
    job.completed_at = Some(chrono::Utc::now());
    update_job(state, &job).await?;
    
    info!("Job {} completed with status {:?}", job_id, job.status);
    Ok(())
}

async fn update_job(state: &ServiceState, job: &ExecutionJob) -> anyhow::Result<()> {
    let mut redis = state.redis.lock().await;
    let job_key = format!("job:{}", job.id);
    let job_json = serde_json::to_string(job)?;
    
    redis::cmd("SET")
        .arg(&job_key)
        .arg(&job_json)
        .query_async::<_, ()>(&mut *redis)
        .await?;
        
    Ok(())
}