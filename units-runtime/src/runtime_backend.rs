use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use units_core::id::UnitsObjectId;
use units_core::objects::UnitsObject;
use units_core::transaction::{RuntimeType, TransactionHash};

//==============================================================================
// INSTRUCTION EXECUTION (simplified)
//==============================================================================

/// Result of a program execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstructionResult {
    /// Success with updated objects
    Success(HashMap<UnitsObjectId, UnitsObject>),
    /// Error with message
    Error(String),
}

/// Execution context for a program or instruction (simplified)
#[derive(Debug, Clone)]
pub struct InstructionContext<'a> {
    /// Transaction hash for the execution
    pub transaction_hash: &'a TransactionHash,
    /// Objects accessible to the instruction
    pub objects: HashMap<UnitsObjectId, UnitsObject>,
}

/// Error returned when execution fails
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Program execution not supported: {0:?}")]
    ProgramExecutionNotSupported(RuntimeType),
    
    #[error("Object not found: {0}")]
    ObjectNotFound(UnitsObjectId),
    
    #[error("Invalid program: {0}")]
    InvalidProgram(UnitsObjectId),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

//==============================================================================
// SIMPLIFIED RUNTIME BACKEND (no abstraction)
//==============================================================================

/// Simplified runtime backend that just returns errors for program execution
pub struct SimpleRuntimeBackend;

impl SimpleRuntimeBackend {
    pub fn new() -> Self {
        Self
    }
    
    /// Get the default runtime type (for compatibility)
    pub fn default_runtime_type(&self) -> RuntimeType {
        RuntimeType::Wasm // Default to WASM for compatibility
    }
    
    /// Execute a program call instruction (always returns error in simplified implementation)
    pub fn execute_program_call<'a>(
        &self,
        program_id: &UnitsObjectId,
        _context: InstructionContext<'a>,
    ) -> Result<HashMap<UnitsObjectId, UnitsObject>, ExecutionError> {
        // In the simplified version, we don't support program execution
        Err(ExecutionError::InvalidProgram(*program_id))
    }
}

//==============================================================================
// LEGACY COMPATIBILITY (for smooth migration)
//==============================================================================

/// Legacy compatibility type alias
pub type RuntimeBackendManager = SimpleRuntimeBackend;

// Legacy compatibility exports for existing code
pub use SimpleRuntimeBackend as WasmRuntimeBackend;
pub use SimpleRuntimeBackend as EbpfRuntimeBackend;

/// Legacy RuntimeBackend trait (simplified)
pub trait RuntimeBackend {
    /// Get the backend's name
    fn name(&self) -> &str;
}

impl RuntimeBackend for SimpleRuntimeBackend {
    fn name(&self) -> &str {
        "Simplified Runtime Backend"
    }
}