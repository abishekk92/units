use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Storage error: {0}")]
    Storage(#[from] units_core_types::error::StorageError),

    #[error("Runtime error: {0}")]
    Runtime(#[from] units_core_types::error::RuntimeError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Object not found: {object_id}")]
    ObjectNotFound { object_id: String },

    #[error("Transaction failed: {reason}")]
    #[allow(dead_code)]
    TransactionFailed { reason: String },

    #[error("Service unavailable: {message}")]
    #[allow(dead_code)]
    ServiceUnavailable { message: String },

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl ServiceError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    pub fn object_not_found(object_id: impl Into<String>) -> Self {
        Self::ObjectNotFound {
            object_id: object_id.into(),
        }
    }

    #[allow(dead_code)]
    pub fn transaction_failed(reason: impl Into<String>) -> Self {
        Self::TransactionFailed {
            reason: reason.into(),
        }
    }

    #[allow(dead_code)]
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            message: message.into(),
        }
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;