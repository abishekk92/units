use anyhow::Result;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::types::error::{ErrorCode, ErrorObject};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use units_core_types::id::UnitsObjectId;
use units_core_types::objects::UnitsObject;
use units_core_types::transaction::{Transaction, TransactionReceipt};

use crate::error::ServiceError;
use crate::service::{UnitsService, HealthStatus};

/// JSON-RPC API trait definition
#[rpc(server)]
pub trait UnitsJsonRpcApi {
    /// Get object by ID
    #[method(name = "getObject")]
    async fn get_object(&self, object_id: String) -> Result<UnitsObject, ErrorObject<'static>>;

    /// Submit transaction
    #[method(name = "submitTransaction")]
    async fn submit_transaction(&self, transaction: Transaction) -> Result<String, ErrorObject<'static>>;

    /// Get transaction by hash
    #[method(name = "getTransaction")]
    async fn get_transaction(&self, tx_hash: String) -> Result<Transaction, ErrorObject<'static>>;

    /// Execute transaction
    #[method(name = "executeTransaction")]
    async fn execute_transaction(&self, tx_hash: String) -> Result<TransactionReceipt, ErrorObject<'static>>;

    /// Get current slot
    #[method(name = "getCurrentSlot")]
    async fn get_current_slot(&self) -> Result<u64, ErrorObject<'static>>;

    /// Health check
    #[method(name = "health")]
    async fn health(&self) -> Result<HealthStatus, ErrorObject<'static>>;

    /// Get version
    #[method(name = "version")]
    async fn version(&self) -> Result<VersionInfo, ErrorObject<'static>>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub commit: String,
    pub build_time: String,
}

/// JSON-RPC server implementation
#[derive(Clone)]
pub struct JsonRpcServerImpl {
    service: UnitsService,
}

impl JsonRpcServerImpl {
    pub fn new(service: UnitsService) -> Self {
        Self { service }
    }

    pub async fn start(&self, addr: SocketAddr) -> Result<impl std::future::Future<Output = ()>> {
        let server = ServerBuilder::default()
            .build(addr)
            .await?;

        let handle = server.start(self.clone().into_rpc());
        
        Ok(async move {
            handle.stopped().await
        })
    }

    fn parse_object_id(id_str: &str) -> Result<UnitsObjectId, ErrorObject<'static>> {
        let bytes = hex::decode(id_str)
            .map_err(|e| ErrorObject::owned(ErrorCode::InvalidParams.code(), format!("Invalid hex: {}", e), None::<()>))?;
        
        if bytes.len() != 32 {
            return Err(ErrorObject::owned(ErrorCode::InvalidParams.code(), "Object ID must be 32 bytes", None::<()>));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(UnitsObjectId::new(array))
    }

    fn parse_tx_hash(hash_str: &str) -> Result<[u8; 32], ErrorObject<'static>> {
        let bytes = hex::decode(hash_str)
            .map_err(|e| ErrorObject::owned(ErrorCode::InvalidParams.code(), format!("Invalid hex: {}", e), None::<()>))?;
        
        if bytes.len() != 32 {
            return Err(ErrorObject::owned(ErrorCode::InvalidParams.code(), "Transaction hash must be 32 bytes", None::<()>));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(array)
    }

    fn map_service_error(err: ServiceError) -> ErrorObject<'static> {
        match err {
            ServiceError::ObjectNotFound { object_id } => {
                ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!("Object not found: {}", object_id),
                    None::<()>,
                )
            }
            ServiceError::InvalidRequest { message } => {
                ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    message,
                    None::<()>,
                )
            }
            ServiceError::TransactionFailed { reason } => {
                ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    format!("Transaction failed: {}", reason),
                    None::<()>,
                )
            }
            _ => {
                ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    err.to_string(),
                    None::<()>,
                )
            }
        }
    }
}

#[async_trait]
impl UnitsJsonRpcApiServer for JsonRpcServerImpl {
    async fn get_object(&self, object_id: String) -> Result<UnitsObject, ErrorObject<'static>> {
        let parsed_id = Self::parse_object_id(&object_id)?;
        self.service
            .get_object(&parsed_id)
            .await
            .map_err(Self::map_service_error)
    }

    async fn submit_transaction(&self, transaction: Transaction) -> Result<String, ErrorObject<'static>> {
        let tx_hash = self.service
            .submit_transaction(transaction)
            .await
            .map_err(Self::map_service_error)?;
        
        Ok(hex::encode(tx_hash))
    }

    async fn get_transaction(&self, tx_hash: String) -> Result<Transaction, ErrorObject<'static>> {
        let parsed_hash = Self::parse_tx_hash(&tx_hash)?;
        self.service
            .get_transaction(&parsed_hash)
            .await
            .map_err(Self::map_service_error)
    }

    async fn execute_transaction(&self, tx_hash: String) -> Result<TransactionReceipt, ErrorObject<'static>> {
        let parsed_hash = Self::parse_tx_hash(&tx_hash)?;
        self.service
            .get_transaction_receipt(&parsed_hash)
            .await
            .map_err(Self::map_service_error)
    }

    async fn get_current_slot(&self) -> Result<u64, ErrorObject<'static>> {
        self.service
            .get_current_slot()
            .await
            .map_err(Self::map_service_error)
    }

    async fn health(&self) -> Result<HealthStatus, ErrorObject<'static>> {
        self.service
            .health_check()
            .await
            .map_err(Self::map_service_error)
    }

    async fn version(&self) -> Result<VersionInfo, ErrorObject<'static>> {
        Ok(VersionInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
            build_time: env!("BUILD_TIME").to_string(),
        })
    }
}