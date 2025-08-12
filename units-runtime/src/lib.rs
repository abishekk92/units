pub mod host_environment;
pub mod mock_runtime;
pub mod receipt_storage;
pub mod runtime;
pub mod runtime_backend;
pub mod transaction_manager;
pub mod verification;

// Re-export the main types for convenience
pub use runtime::Runtime;
pub use units_core::transaction::{TransactionEffect, TransactionReceipt};
// Re-export moved traits from units-storage-impl
pub use units_storage_impl::storage_traits::{TransactionReceiptStorage, UnitsReceiptIterator};

// Re-export types from units-core
pub use units_core::locks::AccessIntent;
pub use units_core::transaction::{ConflictResult, Instruction, Transaction, TransactionHash};

pub use verification::{detect_double_spend, verify_transaction_included, ProofVerifier};

// Re-export runtime backend types
pub use runtime_backend::{
    EbpfRuntimeBackend, ExecutionError, InstructionContext, InstructionResult, RuntimeBackend,
    RuntimeBackendManager, WasmRuntimeBackend,
};

// Re-export host environment types
pub use host_environment::{
    create_standard_host_environment, HostEnvironment, StandardHostEnvironment,
};

// Re-export MockRuntime and InMemoryReceiptStorage for testing
pub use mock_runtime::{InMemoryReceiptStorage, MockRuntime};

// Re-export VerificationResult from units-proofs
pub use units_core::proofs::VerificationResult;
