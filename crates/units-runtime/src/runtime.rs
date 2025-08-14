use crate::vm_executor::{ExecutionContext, VMExecutionError, VMExecutor};
use std::collections::HashMap;
use units_core_types::error::RuntimeError;
use units_core_types::id::UnitsObjectId;
use units_core_types::objects::UnitsObject;
use units_core_types::transaction::{
    CommitmentLevel, ConflictResult, Instruction, Transaction, TransactionHash, TransactionReceipt,
};

/// Runtime for executing transactions and programs in the UNITS system
pub trait Runtime {
    /// Get the VM executors available to this runtime
    fn get_vm_executor(&self, vm_type: units_core_types::objects::VMType) -> Option<Box<dyn VMExecutor>>;

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
    ) -> Result<Vec<crate::vm_executor::ObjectEffect>, VMExecutionError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use units_core_types::id::UnitsObjectId;

    #[test]
    fn test_transaction_receipt_creation() {
        // Create a transaction receipt
        let transaction_hash = [1u8; 32];
        let slot = 42;
        let success = true;
        let timestamp = 123456789;

        let mut receipt = TransactionReceipt::new(transaction_hash, slot, success, timestamp);

        // Verify the receipt fields
        assert_eq!(receipt.transaction_hash, transaction_hash);
        assert_eq!(receipt.slot, slot);
        assert_eq!(receipt.success, success);
        assert_eq!(receipt.timestamp, timestamp);
        assert_eq!(receipt.object_count(), 0);
        assert_eq!(receipt.effects.len(), 0);

        // Add some object proofs
        let object_id1 = UnitsObjectId::unique_id_for_tests();
        let object_id2 = UnitsObjectId::unique_id_for_tests();

        // Create proper UnitsObjectProof instances for testing
        use units_core_types::UnitsObjectProof;
        use units_proofs::current_slot;
        
        let proof1 = UnitsObjectProof::new(
            object_id1.into(),
            [1u8; 32],
            current_slot(),
            vec![1, 2, 3],
            None,
            None,
        );
        
        let proof2 = UnitsObjectProof::new(
            object_id2.into(),
            [2u8; 32],
            current_slot(),
            vec![4, 5, 6],
            None,
            None,
        );

        receipt.add_proof(object_id1, proof1);
        receipt.add_proof(object_id2, proof2);

        // Verify the proofs were added
        assert_eq!(receipt.object_count(), 2);

        // Check if objects exist in the collection
        assert!(receipt.object_proofs.contains_key(&object_id1));

        // Test setting an error
        let error_msg = "Transaction failed".to_string();
        receipt.set_error(error_msg.clone());

        assert_eq!(receipt.success, false);
        assert_eq!(receipt.error_message, Some(error_msg));
    }
}
