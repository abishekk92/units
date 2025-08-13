// New consolidated storage architecture
pub mod storage;
pub mod receipt_storage;
pub mod consolidated_storage;

// Legacy modules (deprecated)
#[deprecated(note = "Use consolidated storage traits instead")]
pub mod deprecated {
    pub mod storage_traits;
    pub mod iterators;
}
pub mod lock_manager;
pub mod wal;

// SQLite temporarily disabled due to legacy interface dependencies
// #[cfg(feature = "sqlite")]
// pub mod sqlite;

// SQLite adapter temporarily disabled due to interface mismatch
// #[cfg(feature = "sqlite")]
// pub mod sqlite_adapter;

//==============================================================================
// NEW CONSOLIDATED EXPORTS
//==============================================================================

// Primary storage traits
pub use storage::{
    ObjectStorage, HistoricalStorage, ProofStorage, WriteAheadLog, LockManager,
    UnitsStorage,
};

// Receipt storage
pub use receipt_storage::{ReceiptStorage, InMemoryReceiptStorage};

// Complete consolidated storage implementations
pub use consolidated_storage::{
    InMemoryObjectStorage, InMemoryProofStorage, InMemoryLockManager,
    NoOpWriteAheadLog, ConsolidatedUnitsStorage,
};

//==============================================================================
// LEGACY EXPORTS (DEPRECATED)
//==============================================================================

#[deprecated(note = "Use ObjectStorage, ProofStorage, and WriteAheadLog instead")]
pub use deprecated::storage_traits::{
    UnitsStorage as LegacyUnitsStorage,
    UnitsStorageProofEngine,
    UnitsWriteAheadLog as LegacyWriteAheadLog,
    TransactionReceiptStorage,
};

#[deprecated(note = "Use standard iterators instead")]
pub use deprecated::storage_traits::{
    UnitsProofIterator, UnitsReceiptIterator, UnitsStateProofIterator,
    UnitsStorageIterator,
};

#[deprecated(note = "Use WriteAheadLog trait instead")]
pub use wal::FileWriteAheadLog;

// Legacy storage implementations (temporarily disabled)
// #[cfg(feature = "sqlite")]
// #[deprecated(note = "Use ConsolidatedUnitsStorage for new code")]
// pub use sqlite::SqliteStorage;

#[cfg(feature = "sqlite")]
#[deprecated(note = "Use storage::LockManager trait instead")]
pub use lock_manager::SqliteLockManager;
