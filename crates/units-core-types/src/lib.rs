pub mod constants;
pub mod error;
pub mod id;
pub mod locks;
pub mod objects;
pub mod transaction;
pub mod scheduler;

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

// Re-export proofs types directly from units-proofs
pub use units_proofs::{
    current_slot,
    SlotNumber,
    StateProof,
    UnitsObjectProof,
    VerificationResult,
    MerkleNode,
    ProofEngine,
};

