//! SQLite Storage Adapter for New Consolidated Traits
//! 
//! This module provides adapters to use the existing SqliteStorage with the new
//! consolidated storage trait interface.

#![cfg(feature = "sqlite")]

use crate::sqlite::SqliteStorage;
use crate::storage::{ObjectStorage, HistoricalStorage, ProofStorage, WriteAheadLog, LockManager};
use crate::receipt_storage::ReceiptStorage;
use crate::storage_traits::UnitsStorage as LegacyUnitsStorage; // Use legacy trait
use std::collections::HashMap;
use std::sync::Arc;
use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::objects::UnitsObject;
use units_core::proofs::{SlotNumber, StateProof, UnitsObjectProof};
use units_core::transaction::TransactionReceipt;

/// Adapter that wraps SqliteStorage to implement the new ObjectStorage trait
pub struct SqliteObjectStorageAdapter {
    storage: Arc<SqliteStorage>,
}

impl SqliteObjectStorageAdapter {
    pub fn new(storage: Arc<SqliteStorage>) -> Self {
        Self { storage }
    }
    
    pub fn storage(&self) -> &Arc<SqliteStorage> {
        &self.storage
    }
}

impl ObjectStorage for SqliteObjectStorageAdapter {
    fn get(&self, id: &UnitsObjectId) -> Result<Option<UnitsObject>, StorageError> {
        self.storage.get(id)
    }
    
    fn set(
        &self,
        object: &UnitsObject,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError> {
        self.storage.set(object, transaction_hash)
    }
    
    fn delete(
        &self,
        id: &UnitsObjectId,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError> {
        self.storage.delete(id, transaction_hash)
    }
    
    fn set_batch(
        &self,
        objects: &[UnitsObject],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        self.storage.set_batch(objects, transaction_hash)
    }
    
    fn delete_batch(
        &self,
        ids: &[UnitsObjectId],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        self.storage.delete_batch(ids, transaction_hash)
    }
    
    fn iter(&self) -> Box<dyn Iterator<Item = Result<UnitsObject, StorageError>> + '_> {
        // Convert the complex async iterator to a simple iterator
        Box::new(self.storage.iter().map(|result| result))
    }
}

impl HistoricalStorage for SqliteObjectStorageAdapter {
    fn get_at_slot(
        &self,
        id: &UnitsObjectId,
        slot: SlotNumber,
    ) -> Result<Option<UnitsObject>, StorageError> {
        self.storage.get_at_slot(id, slot)
    }
    
    fn get_history(
        &self,
        id: &UnitsObjectId,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<(SlotNumber, UnitsObject)>, StorageError> {
        let history_map = self.storage.get_history(id, start_slot, end_slot)?;
        Ok(history_map.into_iter().collect())
    }
    
    fn compact_history(
        &self,
        _before_slot: SlotNumber,
    ) -> Result<usize, StorageError> {
        // SQLite implementation would need to be extended for this
        // For now, return 0 as no-op
        Ok(0)
    }
}

/// Adapter for proof storage functionality
pub struct SqliteProofStorageAdapter {
    storage: Arc<SqliteStorage>,
}

impl SqliteProofStorageAdapter {
    pub fn new(storage: Arc<SqliteStorage>) -> Self {
        Self { storage }
    }
}

impl ProofStorage for SqliteProofStorageAdapter {
    fn store_object_proof(&self, proof: &UnitsObjectProof) -> Result<(), StorageError> {
        self.storage.record_object_proof(proof)
    }
    
    fn get_latest_proof(&self, id: &UnitsObjectId) -> Result<Option<UnitsObjectProof>, StorageError> {
        self.storage.get_latest_proof(id)
    }
    
    fn get_proof_history(
        &self,
        id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<(SlotNumber, UnitsObjectProof)>, StorageError> {
        let iter = self.storage.get_proof_history(id);
        let proofs: Vec<_> = iter
            .filter_map(|result| result.ok())
            .filter(|(slot, _)| {
                if let Some(start) = start_slot {
                    if *slot < start { return false; }
                }
                if let Some(end) = end_slot {
                    if *slot > end { return false; }
                }
                true
            })
            .collect();
        Ok(proofs)
    }
    
    fn store_state_proof(&self, proof: &StateProof) -> Result<(), StorageError> {
        self.storage.record_state_proof(proof)
    }
    
    fn get_state_proof(&self, slot: SlotNumber) -> Result<Option<StateProof>, StorageError> {
        self.storage.get_state_proof(slot)
    }
    
    fn get_state_proof_history(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<StateProof>, StorageError> {
        let iter = self.storage.get_state_proof_history();
        let proofs: Vec<_> = iter
            .filter_map(|result| result.ok())
            .filter(|proof| proof.slot >= start_slot && proof.slot <= end_slot)
            .collect();
        Ok(proofs)
    }
}

/// Adapter for receipt storage functionality
pub struct SqliteReceiptStorageAdapter {
    storage: Arc<SqliteStorage>,
}

impl SqliteReceiptStorageAdapter {
    pub fn new(storage: Arc<SqliteStorage>) -> Self {
        Self { storage }
    }
}

impl ReceiptStorage for SqliteReceiptStorageAdapter {
    fn store_receipt(&self, receipt: &TransactionReceipt) -> Result<(), StorageError> {
        self.storage.store_transaction_receipt(receipt)
    }
    
    fn get_receipt(&self, tx_hash: &[u8; 32]) -> Result<Option<TransactionReceipt>, StorageError> {
        self.storage.get_transaction_receipt(tx_hash)
    }
    
    fn get_receipts_for_slot(&self, slot: SlotNumber) -> Result<Vec<TransactionReceipt>, StorageError> {
        let iter = self.storage.get_transaction_receipts_for_slot(slot);
        iter.collect()
    }
    
    fn get_receipts_range(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        let iter = self.storage.get_transaction_receipts_range(start_slot, end_slot);
        iter.collect()
    }
    
    fn get_receipts_for_object(
        &self,
        _object_id: &UnitsObjectId,
        _start_slot: Option<SlotNumber>,
        _end_slot: Option<SlotNumber>,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        // This would need to be implemented in the SQLite storage
        // For now, return empty as placeholder
        Ok(Vec::new())
    }
    
    fn cleanup_receipts_before(&self, _slot: SlotNumber) -> Result<usize, StorageError> {
        // This would need to be implemented in the SQLite storage
        // For now, return 0 as no-op
        Ok(0)
    }
}

/// Simplified lock guard that wraps the existing lock manager
pub struct SqliteLockGuard<'a> {
    _guard: units_core::locks::ObjectLockGuard<'a, crate::lock_manager::SqliteLockManager>,
}

/// Adapter for lock manager functionality
pub struct SqliteLockManagerAdapter {
    storage: Arc<SqliteStorage>,
}

impl SqliteLockManagerAdapter {
    pub fn new(storage: Arc<SqliteStorage>) -> Self {
        Self { storage }
    }
}

impl LockManager for SqliteLockManagerAdapter {
    type Guard<'a> = SqliteLockGuard<'a> where Self: 'a;
    
    fn lock(&self, id: &UnitsObjectId) -> Result<Self::Guard<'_>, StorageError> {
        let guard = self.storage.lock_manager().lock(*id)?;
        Ok(SqliteLockGuard { _guard: guard })
    }
    
    fn try_lock(&self, id: &UnitsObjectId) -> Result<Option<Self::Guard<'_>>, StorageError> {
        if let Ok(guard) = self.storage.lock_manager().try_lock(*id) {
            Ok(Some(SqliteLockGuard { _guard: guard }))
        } else {
            Ok(None)
        }
    }
    
    fn lock_many(&self, _ids: &[UnitsObjectId]) -> Result<Vec<Self::Guard<'_>>, StorageError> {
        // PersistentLockManager doesn't have lock_many, so this is unimplemented
        // Would need to be added to the trait or implemented here
        Err(StorageError::from("lock_many not implemented for SqliteLockManager"))
    }
}

/// Comprehensive adapter that combines all storage functionality
pub struct ComprehensiveSqliteAdapter {
    objects: SqliteObjectStorageAdapter,
    proofs: SqliteProofStorageAdapter,
    receipts: SqliteReceiptStorageAdapter,
    locks: SqliteLockManagerAdapter,
}

impl ComprehensiveSqliteAdapter {
    pub fn new(storage: SqliteStorage) -> Self {
        let storage_arc = Arc::new(storage);
        Self {
            objects: SqliteObjectStorageAdapter::new(storage_arc.clone()),
            proofs: SqliteProofStorageAdapter::new(storage_arc.clone()),
            receipts: SqliteReceiptStorageAdapter::new(storage_arc.clone()),
            locks: SqliteLockManagerAdapter::new(storage_arc),
        }
    }
    
    pub fn objects(&self) -> &SqliteObjectStorageAdapter {
        &self.objects
    }
    
    pub fn proofs(&self) -> &SqliteProofStorageAdapter {
        &self.proofs
    }
    
    pub fn receipts(&self) -> &SqliteReceiptStorageAdapter {
        &self.receipts
    }
    
    pub fn locks(&self) -> &SqliteLockManagerAdapter {
        &self.locks
    }
}