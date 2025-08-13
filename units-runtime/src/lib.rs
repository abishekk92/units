pub mod host_environment;
pub mod mock_runtime;
pub mod receipt_storage;
pub mod riscv_executor;
pub mod runtime;
pub mod transaction_manager;
pub mod verification;
pub mod vm_executor;

// Re-export the main types for convenience
pub use runtime::Runtime;
pub use units_core::transaction::{TransactionEffect, TransactionReceipt};
// Re-export storage traits and implementations
pub use units_storage::ReceiptStorage;
pub use units_storage_impl::InMemoryReceiptStorage as ReceiptStorageImpl;

// Re-export types from units-core
pub use units_core::locks::AccessIntent;
pub use units_core::transaction::{ConflictResult, Instruction, Transaction, TransactionHash};

pub use verification::{detect_double_spend, verify_transaction_included, ProofVerifier};


// Re-export host environment types
pub use host_environment::{
    create_host_environment, HostEnvironment, StandardHostEnvironment,
};

// Re-export MockRuntime for testing
pub use mock_runtime::MockRuntime;

// Re-export receipt storage for testing  
pub use receipt_storage::InMemoryReceiptStorage;

// Re-export VerificationResult from units-proofs
pub use units_core::proofs::VerificationResult;

// Re-export VM executor types
pub use vm_executor::{ExecutionContext, ObjectEffect, VMExecutionError, VMExecutor};
pub use riscv_executor::{RiscVExecutor, RiscVExecutorConfig};
