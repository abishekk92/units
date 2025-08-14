pub mod constants;
pub mod error;
pub mod id;
pub mod locks;
pub mod objects;
pub mod proofs;
pub mod transaction;
pub mod scheduler;
pub mod storage;

// Re-export the main types for convenience
pub use constants::{
    SYSTEM_LOADER_ID,
    TOKEN_CONTROLLER_ID,
    ACCOUNT_CONTROLLER_ID,
    MODULE_MANAGER_ID,
    is_system_controller,
};
pub use error::StorageError;
pub use id::UnitsObjectId;
pub use objects::{
    VMType,
    ObjectType,
    UnitsObject,
};

// Re-export lock types
pub use locks::{
    AccessIntent,
    LockInfo,
    LockType, 
    ObjectLockGuard,
    PersistentLockManager,
};

// Re-export transaction types
pub use transaction::{
    CommitmentLevel,
    ConflictResult,
    Instruction,
    Transaction,
    TransactionEffect,
    TransactionHash,
    TransactionReceipt,
    ObjectEffect,
};

// Re-export scheduler types
pub use scheduler::{
    ConflictChecker,
    BasicConflictChecker,
};

// Re-export proof types
pub use proofs::{
    Proof,
    SlotNumber,
    StateProof,
    UnitsObjectProof,
    VerificationResult,
    MerkleNode,
    ProofStorageError,
};

// Re-export storage traits
pub use storage::{
    ObjectStorage,
    HistoricalStorage,
    ProofStorage,
    WriteAheadLog,
    ReceiptStorage,
    LockManager,
    UnitsStorage,
};


