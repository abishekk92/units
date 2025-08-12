use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::locks::PersistentLockManager;
use units_core::objects::UnitsObject;
use units_core::proofs::{ProofEngine, SlotNumber, StateProof, UnitsObjectProof};
use units_core::transaction::{CommitmentLevel, TransactionReceipt};

use std::collections::HashMap;
use std::path::Path;

//==============================================================================
// SIMPLIFIED ITERATOR TYPES
//==============================================================================

/// Iterator for objects in storage
pub type ObjectIterator = Box<dyn Iterator<Item = Result<UnitsObject, StorageError>>>;

/// Iterator for object proofs in storage
pub type ProofIterator =
    Box<dyn Iterator<Item = Result<(SlotNumber, UnitsObjectProof), StorageError>>>;

/// Iterator for state proofs in storage
pub type StateProofIterator = Box<dyn Iterator<Item = Result<StateProof, StorageError>>>;

/// Iterator for transaction receipts in storage
pub type ReceiptIterator = Box<dyn Iterator<Item = Result<TransactionReceipt, StorageError>>>;

//==============================================================================
// CONSOLIDATED STORAGE INTERFACE
//==============================================================================

/// Main storage interface for UNITS objects - consolidates all storage functionality
pub trait UnitsStorage {
    /// Get the lock manager for this storage
    fn lock_manager(&self) -> &dyn PersistentLockManager<Error = StorageError>;

    /// Get the proof engine used by this storage
    fn proof_engine(&self) -> &dyn ProofEngine;

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
    fn scan(&self) -> ObjectIterator;

    //--------------------------------------------------------------------------
    // BATCH OPERATIONS
    //--------------------------------------------------------------------------

    /// Store multiple objects in a single transaction
    fn set_batch(
        &self,
        objects: &[UnitsObject],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError>;

    /// Delete multiple objects in a single transaction
    fn delete_batch(
        &self,
        ids: &[UnitsObjectId],
        transaction_hash: [u8; 32],
    ) -> Result<HashMap<UnitsObjectId, UnitsObjectProof>, StorageError>;

    //--------------------------------------------------------------------------
    // PROOF ENGINE OPERATIONS
    //--------------------------------------------------------------------------

    /// Generate a state proof for the current state of all objects
    fn generate_state_proof(&self, slot: Option<SlotNumber>) -> Result<StateProof, StorageError>;

    /// Generate a state proof for the current slot and store it
    fn generate_and_store_state_proof(&self) -> Result<StateProof, StorageError>;

    /// Get the most recent proof for a specific object
    fn get_proof(&self, id: &UnitsObjectId) -> Result<Option<UnitsObjectProof>, StorageError>;

    /// Get all historical proofs for a specific object
    fn get_proof_history(&self, id: &UnitsObjectId) -> ProofIterator;

    /// Get a specific historical proof for an object
    fn get_proof_at_slot(
        &self,
        id: &UnitsObjectId,
        slot: SlotNumber,
    ) -> Result<Option<UnitsObjectProof>, StorageError>;

    /// Get all state proofs
    fn get_state_proofs(&self) -> StateProofIterator;

    /// Get a state proof for a specific slot
    fn get_state_proof_at_slot(&self, slot: SlotNumber)
        -> Result<Option<StateProof>, StorageError>;

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

    //--------------------------------------------------------------------------
    // TRANSACTION RECEIPT OPERATIONS
    //--------------------------------------------------------------------------

    /// Store a transaction receipt
    fn store_receipt(&self, receipt: &TransactionReceipt) -> Result<(), StorageError>;

    /// Get a transaction receipt by transaction hash
    fn get_receipt(&self, hash: &[u8; 32]) -> Result<Option<TransactionReceipt>, StorageError>;

    /// Get all transaction receipts for a specific object
    fn get_receipts_for_object(&self, id: &UnitsObjectId) -> ReceiptIterator;

    /// Get all transaction receipts in a specific slot
    fn get_receipts_in_slot(&self, slot: SlotNumber) -> ReceiptIterator;

    /// Update a transaction's commitment level
    fn update_transaction_commitment(
        &self,
        transaction_hash: &[u8; 32],
        commitment_level: CommitmentLevel,
    ) -> Result<(), StorageError>;

    //--------------------------------------------------------------------------
    // WRITE-AHEAD LOG OPERATIONS (optional - implementations can return not implemented)
    //--------------------------------------------------------------------------

    /// Initialize the write-ahead log
    fn init_wal(&self, _path: &Path) -> Result<(), StorageError> {
        Err(StorageError::Unimplemented(
            "WAL not supported by this storage".to_string(),
        ))
    }

    /// Record an object update in the write-ahead log
    fn record_wal_update(
        &self,
        _object: &UnitsObject,
        _proof: &UnitsObjectProof,
        _transaction_hash: Option<[u8; 32]>,
    ) -> Result<(), StorageError> {
        Err(StorageError::Unimplemented(
            "WAL not supported by this storage".to_string(),
        ))
    }

    /// Record a state proof in the write-ahead log
    fn record_wal_state_proof(&self, _state_proof: &StateProof) -> Result<(), StorageError> {
        Err(StorageError::Unimplemented(
            "WAL not supported by this storage".to_string(),
        ))
    }
}
