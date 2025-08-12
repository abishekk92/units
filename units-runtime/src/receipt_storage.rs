//! Unified receipt storage interface
//! 
//! This module provides a single, clean interface for transaction receipt storage,
//! eliminating the duplication between TransactionReceiptStorage trait and
//! receipt methods in UnitsStorage.

use std::collections::HashMap;
use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::proofs::SlotNumber;
use units_core::transaction::{CommitmentLevel, TransactionHash, TransactionReceipt};

//==============================================================================
// UNIFIED RECEIPT STORAGE
//==============================================================================

/// Unified interface for transaction receipt storage
/// 
/// This replaces the split functionality between TransactionReceiptStorage
/// and UnitsStorage receipt methods.
pub trait ReceiptStorage: Send + Sync {
    //--------------------------------------------------------------------------
    // BASIC OPERATIONS
    //--------------------------------------------------------------------------
    
    /// Store a transaction receipt
    fn store(&self, receipt: &TransactionReceipt) -> Result<(), StorageError>;
    
    /// Get a receipt by transaction hash
    fn get(&self, hash: &TransactionHash) -> Result<Option<TransactionReceipt>, StorageError>;
    
    /// Delete a receipt (for rollback scenarios)
    fn delete(&self, hash: &TransactionHash) -> Result<bool, StorageError>;
    
    /// Check if a receipt exists
    fn exists(&self, hash: &TransactionHash) -> Result<bool, StorageError> {
        Ok(self.get(hash)?.is_some())
    }
    
    //--------------------------------------------------------------------------
    // QUERIES
    //--------------------------------------------------------------------------
    
    /// Get all receipts for a specific object
    fn get_by_object(
        &self,
        object_id: &UnitsObjectId,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Get all receipts in a specific slot
    fn get_by_slot(
        &self,
        slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Get receipts within a slot range
    fn get_by_slot_range(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Get receipts by commitment level
    fn get_by_commitment_level(
        &self,
        level: CommitmentLevel,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    //--------------------------------------------------------------------------
    // UPDATES
    //--------------------------------------------------------------------------
    
    /// Update a receipt's commitment level
    fn update_commitment_level(
        &self,
        hash: &TransactionHash,
        level: CommitmentLevel,
    ) -> Result<(), StorageError> {
        // Default implementation - fetch, update, store
        if let Some(mut receipt) = self.get(hash)? {
            receipt.commitment_level = level;
            self.store(&receipt)
        } else {
            Err(StorageError::ReceiptNotFound(*hash))
        }
    }
    
    //--------------------------------------------------------------------------
    // BATCH OPERATIONS
    //--------------------------------------------------------------------------
    
    /// Store multiple receipts
    fn store_batch(
        &self,
        receipts: &[TransactionReceipt],
    ) -> Result<(), StorageError> {
        // Default implementation - can be overridden for optimization
        for receipt in receipts {
            self.store(receipt)?;
        }
        Ok(())
    }
    
    /// Get multiple receipts by their hashes
    fn get_batch(
        &self,
        hashes: &[TransactionHash],
    ) -> Result<HashMap<TransactionHash, TransactionReceipt>, StorageError> {
        // Default implementation - can be overridden for optimization
        let mut results = HashMap::new();
        for hash in hashes {
            if let Some(receipt) = self.get(hash)? {
                results.insert(*hash, receipt);
            }
        }
        Ok(results)
    }
}

//==============================================================================
// IN-MEMORY IMPLEMENTATION
//==============================================================================

/// Simple in-memory receipt storage for testing
#[derive(Debug, Default)]
pub struct InMemoryReceiptStorage {
    receipts: std::sync::RwLock<HashMap<TransactionHash, TransactionReceipt>>,
    by_object: std::sync::RwLock<HashMap<UnitsObjectId, Vec<TransactionHash>>>,
    by_slot: std::sync::RwLock<HashMap<SlotNumber, Vec<TransactionHash>>>,
}

impl InMemoryReceiptStorage {
    /// Create a new in-memory receipt storage
    pub fn new() -> Self {
        Self::default()
    }
}

impl ReceiptStorage for InMemoryReceiptStorage {
    fn store(&self, receipt: &TransactionReceipt) -> Result<(), StorageError> {
        let hash = receipt.transaction_hash;
        
        // Store the receipt
        self.receipts.write()
            .map_err(|_| StorageError::LockError("Failed to acquire write lock".to_string()))?
            .insert(hash, receipt.clone());
        
        // Update object index
        let mut by_object = self.by_object.write()
            .map_err(|_| StorageError::LockError("Failed to acquire write lock".to_string()))?;
        
        for (object_id, _) in &receipt.object_proofs {
            by_object.entry(*object_id)
                .or_insert_with(Vec::new)
                .push(hash);
        }
        
        // Update slot index
        self.by_slot.write()
            .map_err(|_| StorageError::LockError("Failed to acquire write lock".to_string()))?
            .entry(receipt.slot)
            .or_insert_with(Vec::new)
            .push(hash);
        
        Ok(())
    }
    
    fn get(&self, hash: &TransactionHash) -> Result<Option<TransactionReceipt>, StorageError> {
        Ok(self.receipts.read()
            .map_err(|_| StorageError::LockError("Failed to acquire read lock".to_string()))?
            .get(hash)
            .cloned())
    }
    
    fn delete(&self, hash: &TransactionHash) -> Result<bool, StorageError> {
        // Get the receipt first to update indices
        let receipt = match self.get(hash)? {
            Some(r) => r,
            None => return Ok(false),
        };
        
        // Remove from main storage
        self.receipts.write()
            .map_err(|_| StorageError::LockError("Failed to acquire write lock".to_string()))?
            .remove(hash);
        
        // Update object index
        let mut by_object = self.by_object.write()
            .map_err(|_| StorageError::LockError("Failed to acquire write lock".to_string()))?;
        
        for (object_id, _) in &receipt.object_proofs {
            if let Some(hashes) = by_object.get_mut(object_id) {
                hashes.retain(|h| h != hash);
            }
        }
        
        // Update slot index
        let mut by_slot = self.by_slot.write()
            .map_err(|_| StorageError::LockError("Failed to acquire write lock".to_string()))?;
        
        if let Some(hashes) = by_slot.get_mut(&receipt.slot) {
            hashes.retain(|h| h != hash);
        }
        
        Ok(true)
    }
    
    fn get_by_object(
        &self,
        object_id: &UnitsObjectId,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        let by_object = self.by_object.read()
            .map_err(|_| StorageError::LockError("Failed to acquire read lock".to_string()))?;
        
        let hashes = by_object.get(object_id)
            .map(|v| v.clone())
            .unwrap_or_default();
        
        drop(by_object);
        
        let receipts = self.receipts.read()
            .map_err(|_| StorageError::LockError("Failed to acquire read lock".to_string()))?;
        
        Ok(hashes.into_iter()
            .filter_map(|h| receipts.get(&h).cloned())
            .collect())
    }
    
    fn get_by_slot(
        &self,
        slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        let by_slot = self.by_slot.read()
            .map_err(|_| StorageError::LockError("Failed to acquire read lock".to_string()))?;
        
        let hashes = by_slot.get(&slot)
            .map(|v| v.clone())
            .unwrap_or_default();
        
        drop(by_slot);
        
        let receipts = self.receipts.read()
            .map_err(|_| StorageError::LockError("Failed to acquire read lock".to_string()))?;
        
        Ok(hashes.into_iter()
            .filter_map(|h| receipts.get(&h).cloned())
            .collect())
    }
    
    fn get_by_slot_range(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        let receipts = self.receipts.read()
            .map_err(|_| StorageError::LockError("Failed to acquire read lock".to_string()))?;
        
        Ok(receipts.values()
            .filter(|r| r.slot >= start_slot && r.slot <= end_slot)
            .cloned()
            .collect())
    }
    
    fn get_by_commitment_level(
        &self,
        level: CommitmentLevel,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        let receipts = self.receipts.read()
            .map_err(|_| StorageError::LockError("Failed to acquire read lock".to_string()))?;
        
        Ok(receipts.values()
            .filter(|r| r.commitment_level == level)
            .cloned()
            .collect())
    }
}