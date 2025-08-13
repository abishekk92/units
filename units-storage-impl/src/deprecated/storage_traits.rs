use serde::{Deserialize, Serialize};
use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::locks::PersistentLockManager;
use units_core::objects::UnitsObject;
use units_core::transaction::{CommitmentLevel, TransactionEffect, TransactionReceipt};
use units_core::proofs::{ProofEngine, SlotNumber, StateProof, UnitsObjectProof};

use std::collections::HashMap;
use std::iter::Iterator;
use std::path::Path;

//==============================================================================
// UTILITY FUNCTIONS AND HELPERS
//==============================================================================

/// Utility functions and helper implementations for storage traits
pub mod utils {
    use super::*;
    use std::collections::HashMap;

    /// Default implementation for set_batch operation
    pub fn set_batch<S: UnitsStorage + ?Sized>(
        storage: &S,
        objects: &[UnitsObject],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        let mut proofs = HashMap::new();
        for object in objects {
            let proof = storage.set(object, Some(transaction_hash))?;
            proofs.insert(*object.id(), proof);
        }
        Ok(proofs)
    }

    /// Default implementation for delete_batch operation
    pub fn delete_batch<S: UnitsStorage + ?Sized>(
        storage: &S,
        ids: &[UnitsObjectId],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        let mut proofs = HashMap::new();
        for id in ids {
            let proof = storage.delete(id, Some(transaction_hash))?;
            proofs.insert(*id, proof);
        }
        Ok(proofs)
    }

    /// Default implementation for get_history operation
    pub fn get_history<S: UnitsStorage + ?Sized>(
        storage: &S,
        id: &UnitsObjectId,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<HashMap<SlotNumber, UnitsObject>, StorageError> {
        let mut history = HashMap::new();
        let proofs: Vec<_> = storage
            .get_proof_history(id)
            .map(|r| r.unwrap())
            .filter(|(slot, _)| *slot >= start_slot && *slot <= end_slot)
            .collect();

        if proofs.is_empty() {
            return Ok(history);
        }

        for (slot, _) in proofs {
            if let Some(obj) = storage.get_at_slot(id, slot)? {
                history.insert(slot, obj);
            }
        }
        Ok(history)
    }

    /// Default implementation for get_transaction_history operation
    pub fn get_transaction_history<S: UnitsStorage + ?Sized>(
        storage: &S,
        id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<[u8; 32]>, StorageError> {
        let mut transactions = Vec::new();
        let proofs: Vec<_> = storage
            .get_proof_history(id)
            .map(|r| r.unwrap())
            .filter(|(slot, _)| {
                if let Some(start) = start_slot {
                    if *slot < start {
                        return false;
                    }
                }
                if let Some(end) = end_slot {
                    if *slot > end {
                        return false;
                    }
                }
                true
            })
            .collect();

        for (_, proof) in proofs {
            if let Some(hash) = proof.transaction_hash {
                if !transactions.contains(&hash) {
                    transactions.push(hash);
                }
            }
        }
        Ok(transactions)
    }

    /// Default implementation for execute_transaction_batch operation
    pub fn execute_transaction_batch<S: UnitsStorage + ?Sized>(
        storage: &S,
        objects_to_store: &[UnitsObject],
        objects_to_delete: &[UnitsObjectId],
        transaction_hash: [u8; 32],
        slot: SlotNumber,
    ) -> Result<TransactionReceipt, StorageError> {
        // Get the current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Create a receipt with Processing commitment level
        let mut receipt = TransactionReceipt::with_commitment_level(
            transaction_hash,
            slot,
            true, // Assume success initially
            timestamp,
            CommitmentLevel::Processing,
        );

        // For each object to store, retrieve its current state for the before image
        for object in objects_to_store {
            let before_image = storage.get(object.id())?;
            let effect = TransactionEffect {
                transaction_hash,
                object_id: *object.id(),
                before_image,
                after_image: Some(object.clone()),
            };
            receipt.add_effect(effect);
        }

        // For each object to delete, retrieve its current state for the before image
        for &object_id in objects_to_delete {
            let before_image = storage.get(&object_id)?;
            if let Some(before) = before_image {
                let effect = TransactionEffect {
                    transaction_hash,
                    object_id,
                    before_image: Some(before),
                    after_image: None, // Object will be deleted
                };
                receipt.add_effect(effect);
            }
        }

        // Apply changes
        for object in objects_to_store {
            storage.set(object, Some(transaction_hash))?;
        }

        for &id in objects_to_delete {
            storage.delete(&id, Some(transaction_hash))?;
        }

        Ok(receipt)
    }

    /// Default implementation for commit_transaction operation
    pub fn commit_transaction<S: UnitsStorage + ?Sized>(
        storage: &S,
        transaction_hash: &[u8; 32],
    ) -> Result<(), StorageError> {
        // Get the transaction receipt
        let mut receipt = match storage.get_transaction_receipt(transaction_hash)? {
            Some(r) => r,
            None => return Err(StorageError::TransactionNotFound(*transaction_hash)),
        };

        // Skip if already committed
        if receipt.commitment_level == CommitmentLevel::Committed {
            return Ok(());
        }

        // Verify transaction is in Processing state
        if receipt.commitment_level != CommitmentLevel::Processing {
            return Err(StorageError::InvalidOperation(format!(
                "Cannot commit transaction with commitment level {:?}. Only Processing transactions can be committed.",
                receipt.commitment_level
            )));
        }

        // Clone effects to avoid borrowing issues
        let effects = receipt.effects.clone();

        // Generate proofs for all effects
        for effect in effects {
            if let Some(after) = &effect.after_image {
                // Object was created or modified
                let proof = storage.set(after, Some(*transaction_hash))?;
                let proof_bytes = bincode::serialize(&proof)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                receipt.add_proof(effect.object_id, proof_bytes);
            } else if effect.before_image.is_some() {
                // Object was deleted
                let proof = storage.delete(&effect.object_id, Some(*transaction_hash))?;
                let proof_bytes = bincode::serialize(&proof)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                receipt.add_proof(effect.object_id, proof_bytes);
            }
        }

        // Mark as committed
        receipt.commit();

        // Update the receipt in storage
        storage.update_transaction_commitment(transaction_hash, CommitmentLevel::Committed)?;

        Ok(())
    }

    /// Default implementation for rollback_transaction operation
    pub fn rollback_transaction<S: UnitsStorage + ?Sized>(
        storage: &S,
        transaction_hash: &[u8; 32],
    ) -> Result<bool, StorageError> {
        // Get the transaction receipt
        let receipt = match storage.get_transaction_receipt(transaction_hash)? {
            Some(r) => r,
            None => return Err(StorageError::TransactionNotFound(*transaction_hash)),
        };

        // Check if the transaction can be rolled back
        if !receipt.can_rollback() {
            return Err(StorageError::InvalidOperation(format!(
                "Cannot rollback transaction with commitment level {:?}. Only Processing transactions can be rolled back.",
                receipt.commitment_level
            )));
        }

        // Restore the before image for each effect
        for effect in receipt.effects {
            if let Some(before_image) = effect.before_image {
                // Object existed before transaction, restore its state
                storage.set(&before_image, None)?;
            } else {
                // Object was created in this transaction, delete it
                storage.delete(&effect.object_id, None)?;
            }
        }

        // Mark the transaction as failed
        storage.update_transaction_commitment(transaction_hash, CommitmentLevel::Failed)?;

        Ok(true)
    }

    /// Default implementation for get_history_stats operation
    pub fn get_history_stats() -> Result<HashMap<String, u64>, StorageError> {
        let mut stats = HashMap::new();
        stats.insert("total_objects".to_string(), 0);
        stats.insert("total_proofs".to_string(), 0);
        stats.insert("total_state_proofs".to_string(), 0);
        stats.insert("oldest_slot".to_string(), 0);
        stats.insert("newest_slot".to_string(), 0);
        Ok(stats)
    }
}

//==============================================================================
// ITERATORS
//==============================================================================

/// Iterator implementation for Units types
pub struct UnitsIterator<T, E = StorageError> {
    inner: Box<dyn Iterator<Item = Result<T, E>> + Send + 'static>,
}

/// Async source for iterator - implements the logic to fetch the next item asynchronously
/// This is used to create generic iterators from async code
pub trait AsyncSource<T, E> {
    /// Fetch the next item asynchronously
    fn fetch_next(&mut self) -> futures::future::BoxFuture<'_, Option<Result<T, E>>>;
}

/// Helper to create iterators from async code
pub struct AsyncSourceAdapter<S> {
    source: S,
    rt: std::sync::Arc<tokio::runtime::Runtime>,
}

impl<S> AsyncSourceAdapter<S> {
    /// Create a new adapter from an async source and a runtime
    pub fn new(source: S, rt: std::sync::Arc<tokio::runtime::Runtime>) -> Self {
        Self { source, rt }
    }
    
    /// Convert to a Units iterator
    pub fn into_iterator<T, E>(self) -> UnitsIterator<T, E> 
    where
        T: 'static + Send,
        E: 'static + Send,
        S: AsyncSource<T, E> + 'static + Send,
    {
        let iter = AsyncIteratorWrapper {
            source: self.source, 
            rt: self.rt,
            _marker: std::marker::PhantomData,
        };
        UnitsIterator::from_iter(iter)
    }
}

/// A wrapper that implements Iterator for an AsyncSource
struct AsyncIteratorWrapper<S, T, E> 
where
    S: AsyncSource<T, E>,
    T: 'static,
    E: 'static,
{
    source: S,
    rt: std::sync::Arc<tokio::runtime::Runtime>,
    _marker: std::marker::PhantomData<(T, E)>,
}

impl<S, T, E> Iterator for AsyncIteratorWrapper<S, T, E> 
where
    S: AsyncSource<T, E>,
    T: 'static,
    E: 'static,
{
    type Item = Result<T, E>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.rt.block_on(self.source.fetch_next())
    }
}

impl<T: 'static + Send, E: 'static + Send> UnitsIterator<T, E> {
    /// Create a new empty iterator
    pub fn empty() -> Self {
        let iter = std::iter::empty();
        Self { inner: Box::new(iter) }
    }
    
    /// Create a new iterator from a single result
    pub fn once(item: Result<T, E>) -> Self {
        let iter = std::iter::once(item);
        Self { inner: Box::new(iter) }
    }
    
    /// Create a new iterator from a vector of results
    pub fn from_vec(items: Vec<Result<T, E>>) -> Self {
        let iter = items.into_iter();
        Self { inner: Box::new(iter) }
    }
    
    /// Create a new iterator from a successful vector of items
    pub fn from_items(items: Vec<T>) -> Self 
    where
        E: std::fmt::Debug + std::fmt::Display + Send,
        T: Clone,
    {
        let iter = items.into_iter().map(Ok);
        Self { inner: Box::new(iter) }
    }
    
    /// Create from an existing iterator
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: Iterator<Item = Result<T, E>> + Send + 'static,
    {
        Self { inner: Box::new(iter) }
    }
}

impl<T, E> Iterator for UnitsIterator<T, E> {
    type Item = Result<T, E>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterator for traversing objects in storage
pub trait UnitsStorageIterator: Iterator<Item = Result<UnitsObject, StorageError>> {}

impl UnitsStorageIterator for UnitsIterator<UnitsObject, StorageError> {}

/// Iterator for traversing object proofs in storage
pub trait UnitsProofIterator:
    Iterator<Item = Result<(SlotNumber, UnitsObjectProof), StorageError>>
{
}

impl UnitsProofIterator for UnitsIterator<(SlotNumber, UnitsObjectProof), StorageError> {}

/// Iterator for traversing state proofs in storage
pub trait UnitsStateProofIterator: Iterator<Item = Result<StateProof, StorageError>> {}

impl UnitsStateProofIterator for UnitsIterator<StateProof, StorageError> {}

/// Iterator for traversing transaction receipts in storage
pub trait UnitsReceiptIterator: Iterator<Item = Result<TransactionReceipt, StorageError>> {}

impl UnitsReceiptIterator for UnitsIterator<TransactionReceipt, StorageError> {}

//==============================================================================
// WRITE-AHEAD LOG
//==============================================================================

/// A write-ahead log entry for an object update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALEntry {
    /// The object being updated
    pub object: UnitsObject,
    /// The slot in which this update occurred
    pub slot: SlotNumber,
    /// The proof generated for this update
    pub proof: UnitsObjectProof,
    /// Timestamp of when this update was recorded
    pub timestamp: u64,
    /// Hash of the transaction that led to this update, if any
    pub transaction_hash: Option<[u8; 32]>,
}

/// Write-ahead log for durably recording all updates before they're committed to storage
pub trait UnitsWriteAheadLog {
    /// Initialize the write-ahead log
    fn init(&self, path: &Path) -> Result<(), StorageError>;

    /// Record an object update in the write-ahead log
    fn record_update(
        &self,
        object: &UnitsObject,
        proof: &UnitsObjectProof,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<(), StorageError>;

    /// Record a state proof in the write-ahead log
    fn record_state_proof(&self, state_proof: &StateProof) -> Result<(), StorageError>;

    /// Get an iterator over all WAL entries
    fn iterate_entries(&self) -> Box<dyn Iterator<Item = Result<WALEntry, StorageError>> + '_>;
    
    /// Get an iterator over WAL entries for a specific object
    fn iterate_entries_for_object<'a>(&'a self, object_id: &'a UnitsObjectId) -> Box<dyn Iterator<Item = Result<WALEntry, StorageError>> + 'a> {
        // Default implementation filters all entries
        Box::new(self.iterate_entries().filter(move |result| {
            if let Ok(entry) = result {
                entry.object.id() == object_id
            } else {
                true // Keep errors in the stream
            }
        }))
    }
}

//==============================================================================
// PROOF ENGINE
//==============================================================================

/// Engine for creating and verifying cryptographic proofs
pub trait UnitsStorageProofEngine {
    /// Get the proof engine used by this storage
    fn proof_engine(&self) -> &dyn ProofEngine;

    /// Generate a state proof for the current state of all objects
    fn generate_state_proof(&self, slot: Option<SlotNumber>) -> Result<StateProof, StorageError>;

    /// Get the most recent proof for a specific object
    fn get_proof(&self, id: &UnitsObjectId) -> Result<Option<UnitsObjectProof>, StorageError>;

    /// Get all historical proofs for a specific object
    fn get_proof_history(&self, id: &UnitsObjectId) -> Box<dyn UnitsProofIterator + '_>;

    /// Get a specific historical proof for an object
    fn get_proof_at_slot(
        &self,
        id: &UnitsObjectId,
        slot: SlotNumber,
    ) -> Result<Option<UnitsObjectProof>, StorageError>;

    /// Get all state proofs
    fn get_state_proofs(&self) -> Box<dyn UnitsStateProofIterator + '_>;

    /// Get a state proof for a specific slot
    fn get_state_proof_at_slot(
        &self, 
        slot: SlotNumber
    ) -> Result<Option<StateProof>, StorageError>;

    /// Verify a proof for a specific object
    fn verify_proof(
        &self,
        id: &UnitsObjectId,
        proof: &UnitsObjectProof,
    ) -> Result<bool, StorageError>;

    /// Verify a proof chain for a specific object
    fn verify_proof_chain(
        &self,
        id: &UnitsObjectId,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<bool, StorageError>;
}

//==============================================================================
// TRANSACTION RECEIPTS
//==============================================================================

/// Storage interface for transaction receipts
pub trait TransactionReceiptStorage {
    /// Store a transaction receipt
    fn store_receipt(&self, receipt: &TransactionReceipt) -> Result<(), StorageError>;

    /// Get a transaction receipt by transaction hash
    fn get_receipt(&self, hash: &[u8; 32]) -> Result<Option<TransactionReceipt>, StorageError>;

    /// Get all transaction receipts for a specific object
    fn get_receipts_for_object(&self, id: &UnitsObjectId) -> Box<dyn UnitsReceiptIterator + '_>;

    /// Get all transaction receipts in a specific slot
    fn get_receipts_in_slot(&self, slot: SlotNumber) -> Box<dyn UnitsReceiptIterator + '_>;
}

//==============================================================================
// MAIN STORAGE INTERFACE
//==============================================================================

/// Main storage interface for UNITS objects
pub trait UnitsStorage: UnitsStorageProofEngine + UnitsWriteAheadLog {
    /// Get the lock manager for this storage
    fn lock_manager(&self) -> &dyn PersistentLockManager<Error = StorageError>;

    //--------------------------------------------------------------------------
    // BASIC OPERATIONS
    //--------------------------------------------------------------------------

    /// Get an object by its ID
    fn get(&self, id: &UnitsObjectId) -> Result<Option<UnitsObject>, StorageError>;

    /// Get an object at a specific historical slot
    fn get_at_slot(
        &self,
        id: &UnitsObjectId,
        slot: SlotNumber,
    ) -> Result<Option<UnitsObject>, StorageError>;

    /// Store an object
    fn set(
        &self,
        object: &UnitsObject,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError>;

    /// Delete an object by its ID
    fn delete(
        &self,
        id: &UnitsObjectId,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError>;

    /// Create an iterator to scan through all objects
    fn scan(&self) -> Box<dyn UnitsStorageIterator + '_>;
    
    /// Generate a state proof for the current slot and store it
    fn generate_and_store_state_proof(&self) -> Result<StateProof, StorageError>;

    //--------------------------------------------------------------------------
    // BATCH OPERATIONS
    //--------------------------------------------------------------------------

    /// Store multiple objects in a single transaction
    fn set_batch(
        &self,
        objects: &[UnitsObject],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        utils::set_batch(self, objects, transaction_hash)
    }

    /// Delete multiple objects in a single transaction
    fn delete_batch(
        &self,
        ids: &[UnitsObjectId],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        utils::delete_batch(self, ids, transaction_hash)
    }

    //--------------------------------------------------------------------------
    // HISTORY OPERATIONS
    //--------------------------------------------------------------------------

    /// Get object state history between slots
    fn get_history(
        &self,
        id: &UnitsObjectId,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<HashMap<SlotNumber, UnitsObject>, StorageError> {
        utils::get_history(self, id, start_slot, end_slot)
    }

    /// Get all transactions that affected an object
    fn get_transaction_history(
        &self,
        id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<[u8; 32]>, StorageError> {
        utils::get_transaction_history(self, id, start_slot, end_slot)
    }

    /// Compact historical data up to a specific slot
    fn compact_history(
        &self,
        _before_slot: SlotNumber,
        _preserve_state_proofs: bool,
    ) -> Result<usize, StorageError> {
        // Default implementation that does nothing
        Ok(0)
    }

    /// Get storage statistics about history size
    fn get_history_stats(&self) -> Result<HashMap<String, u64>, StorageError> {
        utils::get_history_stats()
    }

    //--------------------------------------------------------------------------
    // TRANSACTION OPERATIONS
    //--------------------------------------------------------------------------
    
    /// Execute a transaction and generate a receipt
    fn execute_transaction_batch(
        &self,
        objects_to_store: &[UnitsObject],
        objects_to_delete: &[UnitsObjectId],
        transaction_hash: [u8; 32],
        slot: SlotNumber,
    ) -> Result<TransactionReceipt, StorageError> {
        utils::execute_transaction_batch(self, objects_to_store, objects_to_delete, transaction_hash, slot)
    }

    /// Get a transaction receipt by its hash
    fn get_transaction_receipt(
        &self,
        _transaction_hash: &[u8; 32],
    ) -> Result<Option<TransactionReceipt>, StorageError> {
        // Default implementation returns None
        Ok(None)
    }

    /// Update a transaction's commitment level
    fn update_transaction_commitment(
        &self,
        _transaction_hash: &[u8; 32],
        _commitment_level: CommitmentLevel,
    ) -> Result<(), StorageError> {
        Err(StorageError::Unimplemented(
            "Updating transaction commitment level not implemented by this storage".to_string(),
        ))
    }

    /// Commit a transaction, making its changes permanent
    fn commit_transaction(&self, transaction_hash: &[u8; 32]) -> Result<(), StorageError> {
        utils::commit_transaction(self, transaction_hash)
    }

    /// Rollback a transaction, reverting all changes
    fn rollback_transaction(&self, transaction_hash: &[u8; 32]) -> Result<bool, StorageError> {
        utils::rollback_transaction(self, transaction_hash)
    }

    /// Mark a transaction as failed
    fn fail_transaction(&self, transaction_hash: &[u8; 32]) -> Result<(), StorageError> {
        self.update_transaction_commitment(transaction_hash, CommitmentLevel::Failed)
    }
}
