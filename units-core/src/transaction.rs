use crate::id::UnitsObjectId;
use crate::locks::{ObjectLockGuard, PersistentLockManager};
use crate::objects::UnitsObject;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction hash type (32-byte array)
pub type TransactionHash = [u8; 32];

/// The result of a transaction conflict check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResult {
    /// No conflicts detected, transaction can proceed
    NoConflict,
    /// Conflicts detected with these transaction hashes
    Conflict(Vec<TransactionHash>),
    /// Read-only transaction, no conflict possible
    ReadOnly,
}

/// Represents the commitment level of a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitmentLevel {
    /// Transaction is in-flight/processing and can be rolled back
    Processing,
    /// Transaction has been committed and cannot be rolled back
    Committed,
    /// Transaction has failed and cannot be executed again
    Failed,
}

impl Default for CommitmentLevel {
    fn default() -> Self {
        CommitmentLevel::Processing
    }
}

/// Identifies the runtime environment for program execution
///
/// This represents the type of runtime needed to execute program code.
/// We only support runtimes with proper isolation guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    /// WebAssembly virtual machine (using wasmtime, wasmer, etc.)
    Wasm,
    /// eBPF virtual machine
    Ebpf,
}

impl Default for RuntimeType {
    fn default() -> Self {
        RuntimeType::Wasm
    }
}

/// Standard entrypoint name used across all runtimes
///
/// Using a consistent entrypoint name across all runtimes ensures
/// that programs can be executed seamlessly regardless of runtime type.
pub const STANDARD_ENTRYPOINT: &str = "main";

/// Transaction instruction - call into controller entrypoint with target function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    /// The controller kernel module to execute
    pub controller_id: UnitsObjectId,
    
    /// Target function name within the controller (e.g., "transfer", "mint")
    pub target_function: String,
    
    /// Objects this instruction will read/modify (all objects controller needs access to)
    pub target_objects: Vec<UnitsObjectId>,
    
    /// Parameters for the specific function call
    pub params: Vec<u8>,
}

impl Instruction {
    /// Create a new instruction
    pub fn new(
        controller_id: UnitsObjectId,
        target_function: String,
        target_objects: Vec<UnitsObjectId>,
        params: Vec<u8>,
    ) -> Self {
        Self {
            controller_id,
            target_function,
            target_objects,
            params,
        }
    }


    /// Get all target objects for this instruction
    pub fn target_objects(&self) -> &[UnitsObjectId] {
        &self.target_objects
    }

    /// Get the controller ID for this instruction
    pub fn controller_id(&self) -> &UnitsObjectId {
        &self.controller_id
    }

    /// Get the target function name
    pub fn target_function(&self) -> &str {
        &self.target_function
    }

    /// Get the parameters for this instruction
    pub fn params(&self) -> &[u8] {
        &self.params
    }
}

/// Transaction that contains multiple instructions to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// List of instructions to be executed as part of this transaction
    pub instructions: Vec<Instruction>,

    /// The hash of the transaction
    pub hash: TransactionHash,

    /// The commitment level of this transaction
    pub commitment_level: CommitmentLevel,
}

impl Transaction {
    /// Create a new transaction with a Processing commitment level
    pub fn new(instructions: Vec<Instruction>, hash: TransactionHash) -> Self {
        Self {
            instructions,
            hash,
            commitment_level: CommitmentLevel::Processing,
        }
    }

    /// Mark the transaction as committed
    pub fn commit(&mut self) {
        self.commitment_level = CommitmentLevel::Committed;
    }

    /// Mark the transaction as failed
    pub fn fail(&mut self) {
        self.commitment_level = CommitmentLevel::Failed;
    }

    /// Check if the transaction can be rolled back
    pub fn can_rollback(&self) -> bool {
        self.commitment_level == CommitmentLevel::Processing
    }

    /// Acquire all locks needed for this transaction
    /// TODO: Implement with new object model - requires access intent information
    pub fn acquire_locks<'a, M: PersistentLockManager>(
        &self,
        _lock_manager: &'a M,
    ) -> Result<Vec<ObjectLockGuard<'a, M>>, M::Error> {
        // TODO: Implement lock acquisition with new instruction model
        // New model doesn't have object_intents, need to determine access patterns differently
        Ok(Vec::new())
    }

    /// Execute the transaction with automatic lock acquisition and release
    ///
    /// This is a convenience method that:
    /// 1. Acquires all needed locks
    /// 2. Calls the provided execution function
    /// 3. Releases all locks when done
    ///
    /// # Parameters
    /// * `lock_manager` - The persistent lock manager to use
    /// * `exec_fn` - Function that receives a reference to this transaction and performs execution
    ///
    /// # Returns
    /// A result containing the result of the execution function if successful,
    /// or an error if any lock could not be acquired
    pub fn execute_with_locks<'a, M: PersistentLockManager, F, R>(
        &self,
        lock_manager: &'a M,
        exec_fn: F,
    ) -> Result<R, M::Error>
    where
        F: FnOnce(&Self) -> R,
    {
        // Acquire all locks
        let _locks = self.acquire_locks(lock_manager)?;

        // Execute the transaction
        let result = exec_fn(self);

        // Locks are automatically released when _locks goes out of scope
        Ok(result)
    }

    /// Create in-memory locks for testing
    /// TODO: Implement with new object model
    #[cfg(test)]
    pub fn create_in_memory_locks<M: PersistentLockManager>(
        &self,
    ) -> Vec<ObjectLockGuard<'static, M>> {
        Vec::new()
    }

    /// Check if all locks needed for this transaction can be acquired
    /// TODO: Implement with new object model
    pub fn can_acquire_all_locks<M: PersistentLockManager>(
        &self,
        _lock_manager: &M,
    ) -> Result<bool, M::Error> {
        Ok(true)
    }
}

/// Represents the before and after state of a UnitsObject in a transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionEffect {
    /// The transaction that caused this effect
    pub transaction_hash: TransactionHash,
    
    /// The ID of the object affected
    pub object_id: UnitsObjectId,
    
    /// The state of the object before the transaction (None if object was created)
    pub before_image: Option<UnitsObject>,
    
    /// The state of the object after the transaction (None if object was deleted)
    pub after_image: Option<UnitsObject>,
}

/// Alias for transaction effect to maintain API compatibility
pub type ObjectEffect = TransactionEffect;

impl TransactionEffect {
    /// Get the transaction hash for this effect
    pub fn transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
    
    /// Get the object ID for this effect
    pub fn object_id(&self) -> &UnitsObjectId {
        &self.object_id
    }
    
    /// Create a new effect for object creation
    pub fn new_creation(
        transaction_hash: TransactionHash,
        object: UnitsObject,
    ) -> Self {
        Self {
            transaction_hash,
            object_id: *object.id(),
            before_image: None,
            after_image: Some(object),
        }
    }
    
    /// Create a new effect for object deletion
    pub fn new_deletion(
        transaction_hash: TransactionHash,
        object: UnitsObject,
    ) -> Self {
        Self {
            transaction_hash,
            object_id: *object.id(),
            before_image: Some(object),
            after_image: None,
        }
    }
    
    /// Create a new effect for object modification
    pub fn new_modification(
        transaction_hash: TransactionHash,
        before: UnitsObject,
        after: UnitsObject,
    ) -> Self {
        Self {
            transaction_hash,
            object_id: *after.id(),
            before_image: Some(before),
            after_image: Some(after),
        }
    }
    
    /// Check if this effect represents an object creation
    pub fn is_creation(&self) -> bool {
        self.before_image.is_none() && self.after_image.is_some()
    }
    
    /// Check if this effect represents an object deletion
    pub fn is_deletion(&self) -> bool {
        self.before_image.is_some() && self.after_image.is_none()
    }
    
    /// Check if this effect represents an object modification
    pub fn is_modification(&self) -> bool {
        self.before_image.is_some() && self.after_image.is_some()
    }
}


/// A receipt of a processed transaction, containing all proofs of object modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    /// The hash of the transaction that was executed
    pub transaction_hash: TransactionHash,

    /// The slot in which this transaction was processed
    pub slot: u64,

    /// Map of object IDs to their state proofs after the transaction
    /// This is a simplified representation; implementations will use appropriate proof types
    pub object_proofs: HashMap<UnitsObjectId, Vec<u8>>,

    /// Whether the transaction was executed successfully
    pub success: bool,

    /// Timestamp when the transaction was processed
    pub timestamp: u64,

    /// The commitment level of this transaction
    pub commitment_level: CommitmentLevel,

    /// Any error message from the execution (if not successful)
    pub error_message: Option<String>,

    /// Effects of the transaction on objects
    pub effects: Vec<TransactionEffect>,
}

impl TransactionReceipt {
    /// Create a new transaction receipt
    pub fn new(
        transaction_hash: TransactionHash,
        slot: u64,
        success: bool,
        timestamp: u64,
    ) -> Self {
        Self {
            transaction_hash,
            slot,
            object_proofs: HashMap::new(),
            success,
            timestamp,
            commitment_level: if success {
                CommitmentLevel::Committed
            } else {
                CommitmentLevel::Failed
            },
            error_message: None,
            effects: Vec::new(),
        }
    }

    /// Create a new transaction receipt with a specific commitment level
    pub fn with_commitment_level(
        transaction_hash: TransactionHash,
        slot: u64,
        success: bool,
        timestamp: u64,
        commitment_level: CommitmentLevel,
    ) -> Self {
        Self {
            transaction_hash,
            slot,
            object_proofs: HashMap::new(),
            success,
            timestamp,
            commitment_level,
            error_message: None,
            effects: Vec::new(),
        }
    }

    /// Add an object proof to the receipt
    pub fn add_proof(&mut self, object_id: UnitsObjectId, proof: Vec<u8>) {
        self.object_proofs.insert(object_id, proof);
    }

    /// Add an effect to the receipt
    pub fn add_effect(&mut self, effect: TransactionEffect) {
        self.effects.push(effect);
    }

    /// Add a new object effect to the receipt
    pub fn add_object_effect(
        &mut self,
        transaction_hash: TransactionHash,
        object_id: UnitsObjectId,
        before_image: Option<UnitsObject>,
        after_image: Option<UnitsObject>,
    ) {
        let effect = TransactionEffect {
            transaction_hash,
            object_id,
            before_image,
            after_image,
        };
        
        self.effects.push(effect);
    }

    /// Set an error message (used when transaction fails)
    pub fn set_error(&mut self, error: String) {
        self.success = false;
        self.commitment_level = CommitmentLevel::Failed;
        self.error_message = Some(error);
    }

    /// Mark the transaction as committed
    pub fn commit(&mut self) {
        self.commitment_level = CommitmentLevel::Committed;
    }

    /// Mark the transaction as failed
    pub fn fail(&mut self) {
        self.success = false;
        self.commitment_level = CommitmentLevel::Failed;
    }

    /// Check if the transaction can be rolled back
    pub fn can_rollback(&self) -> bool {
        self.commitment_level == CommitmentLevel::Processing
    }

    /// Get the number of objects modified by this transaction
    pub fn object_count(&self) -> usize {
        self.object_proofs.len()
    }
    
    /// Get the total number of effects
    pub fn effect_count(&self) -> usize {
        self.effects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{UnitsObject, VMType};
    use crate::id::UnitsObjectId;
    
    #[test]
    fn test_transaction_effect() {
        // Create an ID for testing
        let id = UnitsObjectId::new([1; 32]);
        let controller_id = UnitsObjectId::new([2; 32]);
        let data = vec![0, 1, 2, 3, 4];
        let transaction_hash = [4; 32];
        
        // Create a data object
        let data_obj = UnitsObject::new_data(id, controller_id, data.clone());
        
        // Create an object creation effect
        let creation_effect = TransactionEffect::new_creation(
            transaction_hash,
            data_obj.clone(),
        );
        
        // Check the effect properties
        assert!(creation_effect.is_creation());
        assert!(!creation_effect.is_deletion());
        assert!(!creation_effect.is_modification());
        assert_eq!(creation_effect.object_id, id);
        assert_eq!(creation_effect.transaction_hash, transaction_hash);
        assert_eq!(creation_effect.before_image, None);
        assert!(creation_effect.after_image.is_some());
        
        // Create a modified object
        let modified_obj = UnitsObject::new_data(id, controller_id, vec![5, 6, 7, 8, 9]);
        
        // Create a modification effect
        let modification_effect = TransactionEffect::new_modification(
            transaction_hash,
            data_obj.clone(),
            modified_obj.clone(),
        );
        
        // Check the effect properties
        assert!(!modification_effect.is_creation());
        assert!(!modification_effect.is_deletion());
        assert!(modification_effect.is_modification());
        
        // Create a deletion effect
        let deletion_effect = TransactionEffect::new_deletion(
            transaction_hash,
            data_obj.clone(),
        );
        
        // Check the effect properties
        assert!(!deletion_effect.is_creation());
        assert!(deletion_effect.is_deletion());
        assert!(!deletion_effect.is_modification());
    }
    
    #[test]
    fn test_transaction_receipt() {
        // Create an ID for testing
        let id = UnitsObjectId::new([1; 32]);
        let controller_id = UnitsObjectId::new([2; 32]);
        let data = vec![0, 1, 2, 3, 4];
        let transaction_hash = [4; 32];
        
        // Create objects
        let data_obj = UnitsObject::new_data(id, controller_id, data.clone());
        
        let exec_obj = UnitsObject::new_executable(
            id,
            controller_id,
            VMType::Wasm,
            data.clone(),
        );
        
        // Create a transaction receipt
        let mut receipt = TransactionReceipt::new(
            transaction_hash,
            1234, // slot
            true, // success
            56789, // timestamp
        );
        
        // Add an object creation effect
        receipt.add_object_effect(
            transaction_hash,
            id,
            None,
            Some(data_obj.clone()),
        );
        
        // Check the receipt contains the effect
        assert_eq!(receipt.effect_count(), 1);
        assert_eq!(receipt.effects.len(), 1);
        
        // Add a modification effect (data to executable)
        receipt.add_object_effect(
            transaction_hash,
            id,
            Some(data_obj.clone()),
            Some(exec_obj.clone()),
        );
        
        // Check that we have two effects
        assert_eq!(receipt.effect_count(), 2);
        
        // Verify the first effect is a creation
        assert!(receipt.effects[0].is_creation());
        
        // Verify the second effect is a modification
        assert!(receipt.effects[1].is_modification());
    }
}
