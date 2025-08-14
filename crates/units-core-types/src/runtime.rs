//! Runtime trait definitions for the UNITS system
//!
//! This module provides the core runtime interfaces without any concrete implementations.

use crate::error::RuntimeError;
use crate::id::UnitsObjectId;
use crate::objects::{UnitsObject, VMType};
use crate::transaction::{
    CommitmentLevel, ConflictResult, Instruction, Transaction, TransactionHash, TransactionReceipt,
};
use std::collections::HashMap;

// Forward declare types that will be defined in vm_executor module
use crate::vm_executor::{ExecutionContext, VMExecutionError, VMExecutor, ObjectEffect};
use crate::verification::Verifier;

/// Runtime for executing transactions and programs in the UNITS system
pub trait Runtime {
    /// Get the VM executors available to this runtime
    fn get_vm_executor(&self, vm_type: VMType) -> Option<Box<dyn VMExecutor>>;

    //--------------------------------------------------------------------------
    // TRANSACTION EXECUTION
    //--------------------------------------------------------------------------

    /// Execute a transaction and return a transaction receipt with proofs
    fn execute_transaction(&self, transaction: Transaction) -> TransactionReceipt;

    /// Try to execute a transaction with conflict checking
    fn try_execute_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<TransactionReceipt, ConflictResult> {
        // Check for conflicts
        match self.check_conflicts(&transaction) {
            Ok(ConflictResult::NoConflict) | Ok(ConflictResult::ReadOnly) => {
                // No conflicts, execute the transaction
                Ok(self.execute_transaction(transaction))
            }
            Ok(conflict) => Err(conflict),
            Err(_) => Err(ConflictResult::Conflict(vec![])),
        }
    }

    /// Check for potential conflicts with pending or recent transactions
    fn check_conflicts(&self, _transaction: &Transaction) -> Result<ConflictResult, RuntimeError> {
        // Default implementation assumes no conflicts
        Ok(ConflictResult::NoConflict)
    }

    //--------------------------------------------------------------------------
    // PROGRAM EXECUTION
    //--------------------------------------------------------------------------

    /// Execute a program call instruction
    fn execute_instruction(
        &self,
        instruction: &Instruction,
        objects: HashMap<UnitsObjectId, UnitsObject>,
        slot: u64,
        timestamp: u64,
    ) -> Result<Vec<ObjectEffect>, VMExecutionError> {
        // Get the controller object 
        let controller = objects.get(&instruction.controller_id)
            .ok_or_else(|| VMExecutionError::InvalidBytecode("Controller object not found".to_string()))?
            .clone();

        // Get VM type from controller
        let vm_type = controller.vm_type()
            .ok_or_else(|| VMExecutionError::InvalidBytecode("Controller is not executable".to_string()))?;

        // Get appropriate VM executor
        let executor = self.get_vm_executor(vm_type)
            .ok_or_else(|| VMExecutionError::UnsupportedVMType(format!("{:?}", vm_type)))?;

        // Create execution context
        let context = ExecutionContext::new(
            instruction.clone(),
            objects,
            slot,
            timestamp,
        );

        // Execute the instruction
        executor.load_and_execute(controller.data(), &context)
    }

    //--------------------------------------------------------------------------
    // TRANSACTION MANAGEMENT
    //--------------------------------------------------------------------------

    /// Get a transaction by its hash
    fn get_transaction(&self, hash: &TransactionHash) -> Option<Transaction>;

    /// Get a transaction receipt by its hash
    fn get_transaction_receipt(&self, hash: &TransactionHash) -> Option<TransactionReceipt>;

    /// Rollback a previously executed transaction
    fn rollback_transaction(&self, transaction_hash: &TransactionHash) -> Result<bool, RuntimeError>;

    /// Update a transaction's commitment level
    fn update_commitment_level(
        &self,
        _transaction_hash: &TransactionHash,
        _commitment_level: CommitmentLevel,
    ) -> Result<(), RuntimeError> {
        Err(RuntimeError::Unimplemented("Updating commitment level not supported".to_string()))
    }

    /// Commit a transaction
    fn commit_transaction(&self, transaction_hash: &TransactionHash) -> Result<(), RuntimeError> {
        self.update_commitment_level(transaction_hash, CommitmentLevel::Committed)
    }

    /// Mark a transaction as failed
    fn fail_transaction(&self, transaction_hash: &TransactionHash) -> Result<(), RuntimeError> {
        self.update_commitment_level(transaction_hash, CommitmentLevel::Failed)
    }

    //--------------------------------------------------------------------------
    // VERIFICATION
    //--------------------------------------------------------------------------

    /// Get the verifier for this runtime
    /// 
    /// All runtime implementations must provide verification capabilities
    /// to ensure transaction and proof integrity.
    fn get_verifier(&self) -> &dyn Verifier;
}