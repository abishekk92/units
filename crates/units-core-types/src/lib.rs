pub mod constants;
pub mod error;
pub mod id;
pub mod locks;
pub mod objects;
pub mod proofs;
pub mod transaction;
pub mod scheduler;
pub mod storage;
pub mod runtime;
pub mod vm_executor;
pub mod transaction_manager;
pub mod verification;

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

// Re-export runtime traits
pub use runtime::Runtime;

// Re-export VM executor traits and types
pub use vm_executor::{
    VMExecutor,
    ExecutionContext,
    ObjectEffect,
    VMExecutionError,
    validate_object_effects,
};

// Re-export transaction manager traits and types
pub use transaction_manager::{
    TransactionManager,
    TransactionFilter,
    TransactionContext,
};

// Re-export verification traits
pub use verification::Verifier;


