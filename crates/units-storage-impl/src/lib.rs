//! UNITS Storage Implementations
//! 
//! This crate provides concrete implementations of the storage traits defined in `units-storage`.
//! 
//! ## Available Implementations
//! 
//! - `InMemoryObjectStorage`: In-memory object storage for testing/development
//! - `InMemoryProofStorage`: In-memory proof storage
//! - `InMemoryReceiptStorage`: In-memory transaction receipt storage
//! - `InMemoryLockManager`: Simple lock manager for development
//! - `FileWriteAheadLog`: File-based write-ahead logging
//! - `ConsolidatedUnitsStorage`: Complete storage solution using composition

pub mod consolidated_storage;
pub mod receipt_storage;
pub mod lock_manager;
pub mod wal;

// Re-export the main storage traits for convenience
pub use units_core_types::{
    ObjectStorage, HistoricalStorage, ProofStorage, WriteAheadLog, 
    LockManager, ReceiptStorage, UnitsStorage,
};

// Export concrete implementations
pub use consolidated_storage::{
    InMemoryObjectStorage, InMemoryProofStorage, NoOpWriteAheadLog, 
    ConsolidatedUnitsStorage,
};

pub use receipt_storage::InMemoryReceiptStorage;
pub use lock_manager::{InMemoryLockManager, SimpleLockGuard};
pub use wal::{FileWriteAheadLog, WALEntry, WALEntryType};