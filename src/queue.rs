use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct RedisQueue {
    conn: Mutex<ConnectionManager>,
    queue_key: String,
}

impl RedisQueue {
    pub fn new(conn: ConnectionManager) -> Self {
        Self {
            conn: Mutex::new(conn),
            queue_key: "syla:execution:queue".to_string(),
        }
    }
    
    pub async fn push_job(&self, job_id: Uuid) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let _: () = conn.lpush(&self.queue_key, job_id.to_string()).await?;
        Ok(())
    }
    
    pub async fn pop_job(&self) -> Result<Option<Uuid>> {
        let mut conn = self.conn.lock().await;
        let result: Option<String> = conn.rpop(&self.queue_key, None).await?;
        result.map(|s| Uuid::parse_str(&s)).transpose().map_err(Into::into)
    }
    
    pub async fn get_queue_length(&self) -> Result<usize> {
        let mut conn = self.conn.lock().await;
        let len: usize = conn.llen(&self.queue_key).await?;
        Ok(len)
    }
}