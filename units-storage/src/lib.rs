//! UNITS Storage Traits
//! 
//! This crate provides the core storage trait definitions for the UNITS (Universal
//! Information Tokenization System) without any concrete implementations.
//! 
//! The traits follow a clean separation of concerns:
//! - `ObjectStorage`: Core object persistence and retrieval
//! - `HistoricalStorage`: Time-travel capabilities for objects  
//! - `ProofStorage`: Cryptographic proof management
//! - `WriteAheadLog`: Optional durability logging
//! - `LockManager`: Object-level locking
//! - `ReceiptStorage`: Transaction receipt management
//! 
//! Concrete implementations are provided by the `units-storage-impl` crate.

use std::collections::HashMap;
use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::objects::UnitsObject;
use units_core::{SlotNumber, StateProof, UnitsObjectProof};
use units_core::transaction::TransactionReceipt;

//==============================================================================
// CORE STORAGE TRAIT
//==============================================================================

/// Core storage interface for UNITS objects
/// 
/// This trait focuses solely on object persistence and retrieval.
/// Transaction management is handled separately by the Runtime.
pub trait ObjectStorage: Send + Sync {
    //--------------------------------------------------------------------------
    // BASIC OPERATIONS
    //--------------------------------------------------------------------------
    
    /// Get an object by its ID
    fn get(&self, id: &UnitsObjectId) -> Result<Option<UnitsObject>, StorageError>;
    
    /// Store an object and return its proof
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
    
    /// Check if an object exists
    fn exists(&self, id: &UnitsObjectId) -> Result<bool, StorageError> {
        Ok(self.get(id)?.is_some())
    }
    
    //--------------------------------------------------------------------------
    // BATCH OPERATIONS
    //--------------------------------------------------------------------------
    
    /// Store multiple objects in a single operation
    fn set_batch(
        &self,
        objects: &[UnitsObject],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        // Default implementation - can be overridden for optimization
        let mut proofs = HashMap::new();
        for object in objects {
            let proof = self.set(object, Some(transaction_hash))?;
            proofs.insert(*object.id(), proof);
        }
        Ok(proofs)
    }
    
    /// Delete multiple objects in a single operation
    fn delete_batch(
        &self,
        ids: &[UnitsObjectId],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError> {
        // Default implementation - can be overridden for optimization
        let mut proofs = HashMap::new();
        for id in ids {
            let proof = self.delete(id, Some(transaction_hash))?;
            proofs.insert(*id, proof);
        }
        Ok(proofs)
    }
    
    //--------------------------------------------------------------------------
    // ITERATION
    //--------------------------------------------------------------------------
    
    /// Iterate over all objects in storage
    /// 
    /// Returns a standard iterator - no complex async adapters
    fn iter(&self) -> Box<dyn Iterator<Item = Result<UnitsObject, StorageError>> + '_>;
    
    /// Iterate over objects matching a filter
    fn iter_filtered<F>(&self, filter: F) -> Box<dyn Iterator<Item = Result<UnitsObject, StorageError>> + '_>
    where
        F: Fn(&UnitsObject) -> bool + 'static,
    {
        Box::new(self.iter().filter(move |result| {
            result.as_ref().map(|obj| filter(obj)).unwrap_or(true)
        }))
    }
}

//==============================================================================
// HISTORICAL STORAGE TRAIT
//==============================================================================

/// Storage with historical state tracking
/// 
/// This trait adds time-travel capabilities to basic storage
pub trait HistoricalStorage: ObjectStorage {
    /// Get an object at a specific historical slot
    fn get_at_slot(
        &self,
        id: &UnitsObjectId,
        slot: SlotNumber,
    ) -> Result<Option<UnitsObject>, StorageError>;
    
    /// Get object state history between slots
    fn get_history(
        &self,
        id: &UnitsObjectId,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<(SlotNumber, UnitsObject)>, StorageError>;
    
    /// Compact historical data before a specific slot
    fn compact_history(
        &self,
        before_slot: SlotNumber,
    ) -> Result<usize, StorageError>;
}

//==============================================================================
// PROOF STORAGE TRAIT
//==============================================================================

/// Storage for cryptographic proofs
/// 
/// Separated from object storage for clarity
pub trait ProofStorage: Send + Sync {
    /// Store an object proof
    fn store_object_proof(
        &self,
        proof: &UnitsObjectProof,
    ) -> Result<(), StorageError>;
    
    /// Get the latest proof for an object
    fn get_latest_proof(
        &self,
        id: &UnitsObjectId,
    ) -> Result<Option<UnitsObjectProof>, StorageError>;
    
    /// Get proof history for an object
    fn get_proof_history(
        &self,
        id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<(SlotNumber, UnitsObjectProof)>, StorageError>;
    
    /// Store a state proof
    fn store_state_proof(
        &self,
        proof: &StateProof,
    ) -> Result<(), StorageError>;
    
    /// Get a state proof for a specific slot
    fn get_state_proof(
        &self,
        slot: SlotNumber,
    ) -> Result<Option<StateProof>, StorageError>;
    
    /// Get state proof history
    fn get_state_proof_history(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<StateProof>, StorageError>;
}

//==============================================================================
// WRITE-AHEAD LOG TRAIT
//==============================================================================

/// Optional write-ahead log for durability
/// 
/// This is a separate concern that can be composed with storage
pub trait WriteAheadLog: Send + Sync {
    /// Record an update before it's committed
    fn record_update(
        &self,
        object: &UnitsObject,
        proof: &UnitsObjectProof,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<(), StorageError>;
    
    /// Record a state proof
    fn record_state_proof(
        &self,
        state_proof: &StateProof,
    ) -> Result<(), StorageError>;
    
    /// Replay the log (for recovery)
    fn replay<F>(&self, callback: F) -> Result<(), StorageError>
    where
        F: FnMut(&UnitsObject, &UnitsObjectProof) -> Result<(), StorageError>;
}

//==============================================================================
// RECEIPT STORAGE TRAIT
//==============================================================================

/// Storage for transaction receipts
/// 
/// This consolidates transaction receipt storage into a single, focused trait
pub trait ReceiptStorage: Send + Sync {
    /// Store a transaction receipt
    fn store_receipt(
        &self,
        receipt: &TransactionReceipt,
    ) -> Result<(), StorageError>;
    
    /// Get a receipt by transaction hash
    fn get_receipt(
        &self,
        tx_hash: &[u8; 32],
    ) -> Result<Option<TransactionReceipt>, StorageError>;
    
    /// Get receipts for a specific slot
    fn get_receipts_for_slot(
        &self,
        slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Get receipts within a slot range
    fn get_receipts_range(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Get receipts affecting a specific object
    fn get_receipts_for_object(
        &self,
        object_id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Delete old receipts before a slot (for cleanup)
    fn cleanup_receipts_before(
        &self,
        slot: SlotNumber,
    ) -> Result<usize, StorageError>;
}

//==============================================================================
// LOCK MANAGER TRAIT
//==============================================================================

/// Simplified lock manager using RAII pattern
pub trait LockManager: Send + Sync {
    /// Lock guard type
    type Guard<'a>: Send + Sync where Self: 'a;
    
    /// Acquire a lock on an object
    fn lock(&self, id: &UnitsObjectId) -> Result<Self::Guard<'_>, StorageError>;
    
    /// Try to acquire a lock without blocking
    fn try_lock(&self, id: &UnitsObjectId) -> Result<Option<Self::Guard<'_>>, StorageError>;
    
    /// Acquire multiple locks atomically (ordered to prevent deadlock)
    fn lock_many(&self, ids: &[UnitsObjectId]) -> Result<Vec<Self::Guard<'_>>, StorageError>;
}

//==============================================================================
// COMPOSED STORAGE TYPE
//==============================================================================

/// Complete storage implementation combining all capabilities
/// 
/// This demonstrates composition over inheritance
pub struct UnitsStorage<O, P, W> {
    pub objects: O,
    pub proofs: P,
    pub wal: Option<W>,
}

impl<O, P, W> UnitsStorage<O, P, W>
where
    O: ObjectStorage,
    P: ProofStorage,
    W: WriteAheadLog,
{
    /// Create a new storage instance
    pub fn new(objects: O, proofs: P, wal: Option<W>) -> Self {
        Self { objects, proofs, wal }
    }
    
    /// Store an object with full proof generation and WAL support
    pub fn store_with_proof(
        &self,
        object: &UnitsObject,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError> {
        // Store the object
        let proof = self.objects.set(object, transaction_hash)?;
        
        // Store the proof
        self.proofs.store_object_proof(&proof)?;
        
        // Record in WAL if available
        if let Some(wal) = &self.wal {
            wal.record_update(object, &proof, transaction_hash)?;
        }
        
        Ok(proof)
    }
}