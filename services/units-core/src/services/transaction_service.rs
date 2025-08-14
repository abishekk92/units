//! Transaction service for managing transaction lifecycle
//!
//! This service handles transaction submission, validation, execution,
//! and coordination with the runtime and storage layers.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use units_core_types::{
    Runtime, ObjectStorage,
    Transaction, TransactionHash, TransactionReceipt,
    ConflictResult,
    UnitsObjectId, UnitsObject, SlotNumber,
};
use units_storage_impl::ConsolidatedUnitsStorage;

use crate::error::{ServiceError, ServiceResult};

/// Transaction pool for managing pending transactions
pub struct TransactionPool {
    /// Pending transactions waiting for execution
    pending: RwLock<HashMap<TransactionHash, Transaction>>,
    /// Transaction receipts by hash
    receipts: RwLock<HashMap<TransactionHash, TransactionReceipt>>,
    /// Maximum pool size
    max_pool_size: usize,
}

impl TransactionPool {
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
            receipts: RwLock::new(HashMap::new()),
            max_pool_size,
        }
    }

    /// Add transaction to pool
    pub async fn add_transaction(&self, transaction: Transaction) -> ServiceResult<TransactionHash> {
        let hash = transaction.hash;
        
        let mut pending = self.pending.write().await;
        if pending.len() >= self.max_pool_size {
            return Err(ServiceError::service_unavailable("Transaction pool is full").into());
        }
        
        pending.insert(hash, transaction);
        Ok(hash)
    }

    /// Get transaction from pool
    pub async fn get_transaction(&self, hash: &TransactionHash) -> Option<Transaction> {
        self.pending.read().await.get(hash).cloned()
    }

    /// Remove transaction from pool
    pub async fn remove_transaction(&self, hash: &TransactionHash) -> Option<Transaction> {
        self.pending.write().await.remove(hash)
    }

    /// Get all pending transactions
    pub async fn get_pending_transactions(&self) -> Vec<Transaction> {
        self.pending.read().await.values().cloned().collect()
    }

    /// Store transaction receipt
    pub async fn store_receipt(&self, receipt: TransactionReceipt) {
        self.receipts.write().await.insert(receipt.transaction_hash, receipt);
    }

    /// Get transaction receipt
    pub async fn get_receipt(&self, hash: &TransactionHash) -> Option<TransactionReceipt> {
        self.receipts.read().await.get(hash).cloned()
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        PoolStats {
            pending_count: self.pending.read().await.len(),
            receipt_count: self.receipts.read().await.len(),
            max_pool_size: self.max_pool_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub pending_count: usize,
    pub receipt_count: usize,
    pub max_pool_size: usize,
}

/// Transaction executor that coordinates with runtime
pub struct TransactionExecutor {
    runtime: Arc<dyn Runtime + Send + Sync>,
    storage: Arc<ConsolidatedUnitsStorage>,
}

impl TransactionExecutor {
    pub fn new(
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
    ) -> Self {
        Self { runtime, storage }
    }

    /// Execute a single transaction
    pub async fn execute_transaction(
        &self,
        transaction: Transaction,
        _slot: SlotNumber,
        _timestamp: u64,
    ) -> ServiceResult<TransactionReceipt> {
        // Check for conflicts first
        match self.runtime.check_conflicts(&transaction)? {
            ConflictResult::Conflict(conflicts) => {
                return Err(ServiceError::transaction_failed(
                    format!("Transaction conflicts with: {:?}", conflicts)
                ));
            }
            ConflictResult::ReadOnly => {
                // Read-only transactions can proceed without locks
            }
            ConflictResult::NoConflict => {
                // No conflicts, proceed with execution
            }
        }

        // Gather all required objects for the transaction
        let mut objects = HashMap::new();
        for instruction in &transaction.instructions {
            // Load controller object
            let controller = self.load_object(&instruction.controller_id).await?;
            objects.insert(instruction.controller_id, controller);

            // Load target objects
            for input_id in &instruction.target_objects {
                if !objects.contains_key(input_id) {
                    let obj = self.load_object(input_id).await?;
                    objects.insert(*input_id, obj);
                }
            }
        }

        // Execute through runtime
        let receipt = self.runtime.execute_transaction(transaction);

        // Apply effects to storage if successful
        if receipt.success {
            for (object_id, proof) in &receipt.object_proofs {
                // The proof contains the new state that was applied
                // In a real implementation, we'd verify the proof here
                let _ = object_id;
                let _ = proof;
            }
        }

        Ok(receipt)
    }

    /// Execute a batch of transactions
    pub async fn execute_batch(
        &self,
        transactions: Vec<Transaction>,
        slot: SlotNumber,
        timestamp: u64,
    ) -> Vec<ServiceResult<TransactionReceipt>> {
        let mut results = Vec::new();

        for transaction in transactions {
            let result = self.execute_transaction(transaction, slot, timestamp).await;
            results.push(result);
        }

        results
    }

    /// Load object from storage
    async fn load_object(&self, id: &UnitsObjectId) -> ServiceResult<UnitsObject> {
        use units_core_types::UnitsStorage;
        self.storage
            .objects()
            .get(id)
            .map_err(ServiceError::Storage)?
            .ok_or_else(|| ServiceError::object_not_found(hex::encode(id.bytes())))
    }
}

/// Main transaction service that combines pool and executor
pub struct TransactionService {
    pool: Arc<TransactionPool>,
    executor: Arc<TransactionExecutor>,
    slot_number: Arc<RwLock<SlotNumber>>,
}

impl TransactionService {
    pub fn new(
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
        max_pool_size: usize,
    ) -> Self {
        let pool = Arc::new(TransactionPool::new(max_pool_size));
        let executor = Arc::new(TransactionExecutor::new(runtime, storage));
        
        Self {
            pool,
            executor,
            slot_number: Arc::new(RwLock::new(0)),
        }
    }

    /// Submit a new transaction
    pub async fn submit_transaction(&self, transaction: Transaction) -> ServiceResult<TransactionHash> {
        // Validate transaction
        self.validate_transaction(&transaction)?;
        
        // Add to pool
        let hash = self.pool.add_transaction(transaction).await?;
        
        Ok(hash)
    }

    /// Execute transactions in the current slot
    pub async fn execute_slot_transactions(&self) -> ServiceResult<Vec<TransactionReceipt>> {
        let slot = *self.slot_number.read().await;
        let timestamp = chrono::Utc::now().timestamp() as u64;
        
        // Get all pending transactions
        let transactions = self.pool.get_pending_transactions().await;
        
        let mut receipts = Vec::new();
        for transaction in transactions {
            let hash = transaction.hash;
            
            match self.executor.execute_transaction(transaction, slot, timestamp).await {
                Ok(receipt) => {
                    // Remove from pool and store receipt
                    self.pool.remove_transaction(&hash).await;
                    self.pool.store_receipt(receipt.clone()).await;
                    receipts.push(receipt);
                }
                Err(e) => {
                    // Log error but continue with other transactions
                    log::error!("Failed to execute transaction {}: {:?}", hex::encode(hash), e);
                }
            }
        }
        
        Ok(receipts)
    }

    /// Get transaction from pool or receipts
    pub async fn get_transaction(&self, hash: &TransactionHash) -> ServiceResult<Transaction> {
        self.pool.get_transaction(hash).await
            .ok_or_else(|| ServiceError::object_not_found(hex::encode(hash)))
    }

    /// Get transaction receipt
    pub async fn get_receipt(&self, hash: &TransactionHash) -> ServiceResult<TransactionReceipt> {
        self.pool.get_receipt(hash).await
            .ok_or_else(|| ServiceError::object_not_found(hex::encode(hash)))
    }

    /// Advance to next slot
    pub async fn advance_slot(&self) -> ServiceResult<SlotNumber> {
        let mut slot = self.slot_number.write().await;
        *slot += 1;
        Ok(*slot)
    }

    /// Get current slot
    pub async fn current_slot(&self) -> SlotNumber {
        *self.slot_number.read().await
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self) -> PoolStats {
        self.pool.get_stats().await
    }

    /// Validate transaction before submission
    fn validate_transaction(&self, transaction: &Transaction) -> ServiceResult<()> {
        if transaction.instructions.is_empty() {
            return Err(ServiceError::invalid_request("Transaction has no instructions"));
        }

        // Additional validation logic
        for (i, instruction) in transaction.instructions.iter().enumerate() {
            if instruction.target_objects.is_empty() && instruction.params.is_empty() {
                return Err(ServiceError::invalid_request(
                    format!("Instruction {} has no inputs or data", i)
                ));
            }
        }

        Ok(())
    }
}