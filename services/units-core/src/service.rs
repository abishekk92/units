use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use units_core_types::id::UnitsObjectId;
use units_core_types::objects::UnitsObject;
use units_core_types::transaction::{Transaction, TransactionReceipt};
use units_storage::ObjectStorage;
use units_storage_impl::ConsolidatedUnitsStorage;
use units_runtime::Runtime;

use crate::config::Config;
use crate::error::{ServiceError, ServiceResult};

/// Core UNITS service that handles business logic
#[derive(Clone)]
pub struct UnitsService {
    storage: Arc<ConsolidatedUnitsStorage>,
    #[allow(dead_code)]
    runtime: Arc<dyn Runtime + Send + Sync>,
    #[allow(dead_code)]
    config: Config,
    // In-memory transaction pool for demonstration
    transaction_pool: Arc<RwLock<HashMap<[u8; 32], Transaction>>>,
}

impl UnitsService {
    pub fn new(
        storage: Arc<ConsolidatedUnitsStorage>,
        runtime: Arc<dyn Runtime + Send + Sync>,
        config: Config,
    ) -> Self {
        Self {
            storage,
            runtime,
            config,
            transaction_pool: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get object by ID
    pub async fn get_object(&self, object_id: &UnitsObjectId) -> ServiceResult<UnitsObject> {
        let object = self
            .storage
            .inner()
            .objects
            .get(object_id)
            .map_err(ServiceError::Storage)?
            .ok_or_else(|| ServiceError::object_not_found(hex::encode(object_id.bytes())))?;
        
        Ok(object)
    }

    /// Submit transaction to the transaction pool
    pub async fn submit_transaction(&self, transaction: Transaction) -> ServiceResult<[u8; 32]> {
        // Simple hash for demo - in real implementation would use proper transaction hashing
        let tx_hash = [0u8; 32]; // mock hash
        
        // Basic validation
        if transaction.instructions.is_empty() {
            return Err(ServiceError::invalid_request("Transaction has no instructions"));
        }

        // Add to transaction pool
        {
            let mut pool = self.transaction_pool.write().await;
            pool.insert(tx_hash, transaction);
        }

        Ok(tx_hash)
    }

    /// Get transaction from pool
    pub async fn get_transaction(&self, tx_hash: &[u8; 32]) -> ServiceResult<Transaction> {
        let pool = self.transaction_pool.read().await;
        pool.get(tx_hash)
            .cloned()
            .ok_or_else(|| ServiceError::object_not_found(hex::encode(tx_hash)))
    }

    /// Execute a transaction (simplified - normally would be handled by consensus)
    pub async fn execute_transaction(&self, tx_hash: &[u8; 32]) -> ServiceResult<TransactionReceipt> {
        let _transaction = self.get_transaction(tx_hash).await?;
        
        // For now, just return a mock receipt
        // In a real implementation, this would:
        // 1. Execute transaction through runtime
        // 2. Apply effects to storage
        // 3. Generate proofs
        // 4. Store receipt
        let receipt = TransactionReceipt {
            transaction_hash: *tx_hash,
            slot: 1, // mock slot
            timestamp: chrono::Utc::now().timestamp() as u64,
            success: true,
            effects: vec![], // would contain actual effects
            object_proofs: std::collections::HashMap::new(),  // would contain actual proofs
            commitment_level: units_core_types::transaction::CommitmentLevel::Committed,
            error_message: None,
        };

        Ok(receipt)
    }

    /// Get current slot number
    pub async fn get_current_slot(&self) -> ServiceResult<u64> {
        // Mock implementation - would query actual slot from consensus
        Ok(1)
    }

    /// Get object count
    pub async fn get_object_count(&self) -> ServiceResult<u64> {
        // This would require adding a count method to storage
        // For now, return mock value
        Ok(0)
    }

    /// Health check
    pub async fn health_check(&self) -> ServiceResult<HealthStatus> {
        Ok(HealthStatus {
            status: "healthy".to_string(),
            slot: self.get_current_slot().await?,
            object_count: self.get_object_count().await?,
            pending_transactions: self.transaction_pool.read().await.len() as u64,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub slot: u64,
    pub object_count: u64,
    pub pending_transactions: u64,
}