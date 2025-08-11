pub mod lock_manager;
pub mod storage_traits;

#[cfg(feature = "sqlite")]
pub mod sqlite;

// Re-export the main types for convenience
pub use storage_traits::{
    ObjectIterator, ProofIterator, ReceiptIterator, StateProofIterator,
    UnitsStorage,
};


// Re-export lock manager implementations
#[cfg(feature = "sqlite")]
pub use lock_manager::SqliteLockManager;

// Re-export the storage implementations
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStorage;
