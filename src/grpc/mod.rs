pub mod server;

use tonic::{Request, Response, Status};
use tracing::{info, warn, error};

// Re-export generated types
pub mod proto {
    pub mod syla {
        pub mod execution {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/syla.execution.v1.rs"));
            }
        }
        pub mod common {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/syla.common.v1.rs"));
            }
        }
    }
}

pub use proto::syla::execution::v1::*;
pub use proto::syla::common::v1::{HealthCheckRequest, HealthCheckResponse, HealthStatus, PageRequest, PageResponse};

// Helper trait for converting errors to gRPC status
pub trait IntoStatus {
    fn into_status(self) -> Status;
}

impl IntoStatus for anyhow::Error {
    fn into_status(self) -> Status {
        error!("Error: {:?}", self);
        Status::internal(format!("Internal error: {}", self))
    }
}

impl IntoStatus for sqlx::Error {
    fn into_status(self) -> Status {
        error!("Database error: {:?}", self);
        match self {
            sqlx::Error::RowNotFound => Status::not_found("Resource not found"),
            _ => Status::internal("Database error"),
        }
    }
}

impl IntoStatus for redis::RedisError {
    fn into_status(self) -> Status {
        error!("Redis error: {:?}", self);
        Status::internal("Queue error")
    }
}

// Authentication interceptor
#[derive(Clone)]
pub struct AuthInterceptor {
    auth_service_url: String,
}

impl AuthInterceptor {
    pub fn new(auth_service_url: String) -> Self {
        Self { auth_service_url }
    }
    
    pub async fn intercept(&self, req: Request<()>) -> Result<Request<()>, Status> {
        // Check for authorization header
        let auth_header = req
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;
        
        let auth_str = auth_header
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;
        
        // Extract bearer token
        let token = auth_str
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("Invalid authorization format"))?;
        
        // TODO: Validate token with auth service
        // For now, just check that it's not empty
        if token.is_empty() {
            return Err(Status::unauthenticated("Invalid token"));
        }
        
        info!("Authenticated request");
        Ok(req)
    }
}