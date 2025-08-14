pub mod host_environment;
pub mod mock_runtime;
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
pub use units_storage_impl::InMemoryReceiptStorage;

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


// Re-export VerificationResult from units-proofs
pub use units_core::VerificationResult;

// Re-export VM executor types
pub use vm_executor::{ExecutionContext, ObjectEffect, VMExecutionError, VMExecutor};
pub use riscv_executor::{RiscVExecutor, RiscVExecutorConfig};
