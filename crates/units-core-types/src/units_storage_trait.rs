//! Unified storage trait for UNITS
//!
//! This trait combines all storage capabilities into a single interface
//! for convenience when working with complete storage implementations.

use crate::{
    ObjectStorage, HistoricalStorage, ProofStorage, 
    WriteAheadLog, ReceiptStorage, LockManager,
};

/// Unified storage trait combining all storage capabilities
pub trait UnitsStorage: Send + Sync {
    /// Object storage implementation
    type Objects: ObjectStorage;
    
    /// Historical storage implementation
    type Historical: HistoricalStorage;
    
    /// Proof storage implementation
    type Proofs: ProofStorage;
    
    /// Write-ahead log implementation
    type WAL: WriteAheadLog;
    
    /// Receipt storage implementation
    type Receipts: ReceiptStorage;
    
    /// Lock manager implementation
    type Locks: LockManager;
    
    /// Get object storage
    fn objects(&self) -> &Self::Objects;
    
    /// Get historical storage
    fn historical(&self) -> &Self::Historical;
    
    /// Get proof storage
    fn proofs(&self) -> &Self::Proofs;
    
    /// Get write-ahead log
    fn wal(&self) -> Option<&Self::WAL>;
    
    /// Get receipt storage
    fn receipts(&self) -> &Self::Receipts;
    
    /// Get lock manager
    fn locks(&self) -> &Self::Locks;
}