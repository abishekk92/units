//! Transaction management traits for the UNITS runtime
//! 
//! This module consolidates all transaction-related operations that were previously
//! split between Storage and Runtime traits.

use std::collections::HashMap;
use crate::error::{RuntimeError, StorageError};
use crate::id::UnitsObjectId;
use crate::objects::UnitsObject;
use crate::{SlotNumber, UnitsObjectProof};
use crate::transaction::{
    CommitmentLevel, ConflictResult, Transaction, TransactionEffect, 
    TransactionHash, TransactionReceipt
};

//==============================================================================
// TRANSACTION MANAGER TRAIT
//==============================================================================

/// Centralized transaction management
/// 
/// This trait consolidates all transaction operations previously split between
/// Storage and Runtime, providing a single source of truth for transaction handling.
pub trait TransactionManager: Send + Sync {
    //--------------------------------------------------------------------------
    // TRANSACTION EXECUTION
    //--------------------------------------------------------------------------
    
    /// Execute a transaction and return a receipt
    fn execute_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<TransactionReceipt, RuntimeError>;
    
    /// Execute a batch of transactions atomically
    fn execute_transaction_batch(
        &self,
        transactions: &[Transaction],
    ) -> Result<Vec<TransactionReceipt>, RuntimeError> {
        // Default implementation - can be overridden for optimization
        let mut receipts = Vec::new();
        for tx in transactions {
            receipts.push(self.execute_transaction(tx)?);
        }
        Ok(receipts)
    }
    
    //--------------------------------------------------------------------------
    // TRANSACTION STORAGE
    //--------------------------------------------------------------------------
    
    /// Store a transaction
    fn store_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<(), StorageError>;
    
    /// Get a transaction by its hash
    fn get_transaction(
        &self,
        hash: &TransactionHash,
    ) -> Result<Option<Transaction>, StorageError>;
    
    /// Store a transaction receipt
    fn store_receipt(
        &self,
        receipt: &TransactionReceipt,
    ) -> Result<(), StorageError>;
    
    /// Get a transaction receipt by its hash
    fn get_receipt(
        &self,
        hash: &TransactionHash,
    ) -> Result<Option<TransactionReceipt>, StorageError>;
    
    //--------------------------------------------------------------------------
    // TRANSACTION LIFECYCLE
    //--------------------------------------------------------------------------
    
    /// Update a transaction's commitment level
    fn update_commitment_level(
        &self,
        hash: &TransactionHash,
        level: CommitmentLevel,
    ) -> Result<(), RuntimeError>;
    
    /// Commit a transaction (finalize it)
    fn commit_transaction(
        &self,
        hash: &TransactionHash,
    ) -> Result<(), RuntimeError> {
        self.update_commitment_level(hash, CommitmentLevel::Committed)
    }
    
    /// Mark a transaction as failed
    fn fail_transaction(
        &self,
        hash: &TransactionHash,
    ) -> Result<(), RuntimeError> {
        self.update_commitment_level(hash, CommitmentLevel::Failed)
    }
    
    /// Rollback a transaction (if possible)
    fn rollback_transaction(
        &self,
        hash: &TransactionHash,
    ) -> Result<bool, RuntimeError>;
    
    //--------------------------------------------------------------------------
    // CONFLICT DETECTION
    //--------------------------------------------------------------------------
    
    /// Check for conflicts with pending transactions
    fn check_conflicts(
        &self,
        transaction: &Transaction,
    ) -> Result<ConflictResult, RuntimeError>;
    
    /// Try to execute a transaction with conflict checking
    fn try_execute_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<TransactionReceipt, RuntimeError> {
        // Check for conflicts first
        match self.check_conflicts(transaction)? {
            ConflictResult::NoConflict | ConflictResult::ReadOnly => {
                // No conflicts, execute the transaction
                self.execute_transaction(transaction)
            }
            ConflictResult::Conflict(_conflicting_ids) => {
                Err(RuntimeError::TransactionConflict(
                    transaction.hash,
                    vec![], // Convert conflicting transaction hashes to object IDs
                ))
            }
        }
    }
    
    //--------------------------------------------------------------------------
    // TRANSACTION HISTORY
    //--------------------------------------------------------------------------
    
    /// Get all transactions for a specific object
    fn get_transactions_for_object(
        &self,
        id: &UnitsObjectId,
    ) -> Result<Vec<TransactionHash>, StorageError>;
    
    /// Get all transactions in a specific slot
    fn get_transactions_in_slot(
        &self,
        slot: SlotNumber,
    ) -> Result<Vec<TransactionHash>, StorageError>;
    
    /// Get transaction history with filters
    fn get_transaction_history(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<(TransactionHash, TransactionReceipt)>, StorageError>;
}

//==============================================================================
// TRANSACTION FILTER
//==============================================================================

/// Filter criteria for querying transaction history
#[derive(Debug, Clone, Default)]
pub struct TransactionFilter {
    /// Filter by object IDs
    pub object_ids: Option<Vec<UnitsObjectId>>,
    
    /// Filter by slot range
    pub start_slot: Option<SlotNumber>,
    pub end_slot: Option<SlotNumber>,
    
    /// Filter by commitment level
    pub commitment_level: Option<CommitmentLevel>,
    
    /// Filter by success status
    pub success_only: bool,
    
    /// Maximum number of results
    pub limit: Option<usize>,
}

impl TransactionFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Filter by object ID
    pub fn with_object(mut self, id: UnitsObjectId) -> Self {
        self.object_ids.get_or_insert_with(Vec::new).push(id);
        self
    }
    
    /// Filter by slot range
    pub fn with_slot_range(mut self, start: SlotNumber, end: SlotNumber) -> Self {
        self.start_slot = Some(start);
        self.end_slot = Some(end);
        self
    }
    
    /// Filter by commitment level
    pub fn with_commitment_level(mut self, level: CommitmentLevel) -> Self {
        self.commitment_level = Some(level);
        self
    }
    
    /// Filter for successful transactions only
    pub fn success_only(mut self) -> Self {
        self.success_only = true;
        self
    }
    
    /// Limit the number of results
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

//==============================================================================
// TRANSACTION EXECUTION CONTEXT
//==============================================================================

/// Context for executing a transaction
/// 
/// This encapsulates all the state needed during transaction execution
pub struct TransactionContext {
    /// The transaction being executed
    pub transaction: Transaction,
    
    /// Current slot number
    pub slot: SlotNumber,
    
    /// Objects affected by this transaction
    pub objects: HashMap<UnitsObjectId, UnitsObject>,
    
    /// Proofs generated during execution
    pub proofs: HashMap<UnitsObjectId, UnitsObjectProof>,
    
    /// Effects of the transaction
    pub effects: Vec<TransactionEffect>,
    
    /// Whether the transaction has been rolled back
    pub rolled_back: bool,
}

impl TransactionContext {
    /// Create a new transaction context
    pub fn new(transaction: Transaction, slot: SlotNumber) -> Self {
        Self {
            transaction,
            slot,
            objects: HashMap::new(),
            proofs: HashMap::new(),
            effects: Vec::new(),
            rolled_back: false,
        }
    }
    
    /// Add an object to the context
    pub fn add_object(&mut self, object: UnitsObject) {
        self.objects.insert(*object.id(), object);
    }
    
    /// Add a proof to the context
    pub fn add_proof(&mut self, id: UnitsObjectId, proof: UnitsObjectProof) {
        self.proofs.insert(id, proof);
    }
    
    /// Add an effect to the context
    pub fn add_effect(&mut self, effect: TransactionEffect) {
        self.effects.push(effect);
    }
    
    /// Mark the transaction as rolled back
    pub fn rollback(&mut self) {
        self.rolled_back = true;
    }
    
    /// Create a transaction receipt from this context
    pub fn into_receipt(self, success: bool, timestamp: u64) -> TransactionReceipt {
        let mut receipt = TransactionReceipt::new(
            self.transaction.hash,
            self.slot,
            success,
            timestamp,
        );
        
        // Add proofs to the receipt
        for (id, proof) in self.proofs {
            receipt.add_proof(id, proof);
        }
        
        // Add effects
        receipt.effects = self.effects;
        
        // Set commitment level
        receipt.commitment_level = if success {
            CommitmentLevel::Processing
        } else {
            CommitmentLevel::Failed
        };
        
        receipt
    }
}