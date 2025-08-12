use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use units_core::id::UnitsObjectId;
use units_core::objects::{UnitsObject, VMType};
use units_core::transaction::{Instruction, TransactionHash};

/// Complete context provided to controller during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// The instruction being executed
    pub instruction: Instruction,
    
    /// Objects the controller can read/modify (pre-loaded from storage)
    /// Controllers can read any object but only modify objects they control
    pub objects: HashMap<UnitsObjectId, UnitsObject>,
    
    /// Current slot number
    pub slot: u64,
    
    /// Current timestamp
    pub timestamp: u64,
    
    /// Host environment variables
    pub env_vars: HashMap<String, String>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(
        instruction: Instruction,
        objects: HashMap<UnitsObjectId, UnitsObject>,
        slot: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            instruction,
            objects,
            slot,
            timestamp,
            env_vars: HashMap::new(),
        }
    }

    /// Get objects that this controller can modify (it controls)
    pub fn writable_objects(&self) -> impl Iterator<Item = (&UnitsObjectId, &UnitsObject)> {
        self.objects.iter().filter(|(_, obj)| {
            obj.controller_id() == &self.instruction.controller_id
        })
    }
    
    /// Get all objects (read-only + writable)
    pub fn all_objects(&self) -> &HashMap<UnitsObjectId, UnitsObject> {
        &self.objects
    }

    /// Add an environment variable
    pub fn add_env_var(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }
}

/// Effect of controller execution on a single object
/// Represents before/after state for one object in an instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectEffect {
    /// The object that was modified
    pub object_id: UnitsObjectId,
    
    /// State before instruction execution (None if object was created)
    pub before_image: Option<UnitsObject>,
    
    /// State after instruction execution (None if object was deleted)  
    pub after_image: Option<UnitsObject>,
}

impl ObjectEffect {
    /// Create new object effect
    pub fn creation(object: UnitsObject) -> Self {
        Self {
            object_id: object.id,
            before_image: None,
            after_image: Some(object),
        }
    }
    
    /// Modify existing object effect
    pub fn modification(before: UnitsObject, after: UnitsObject) -> Self {
        Self {
            object_id: after.id,
            before_image: Some(before),
            after_image: Some(after),
        }
    }
    
    /// Delete object effect
    pub fn deletion(object: UnitsObject) -> Self {
        Self {
            object_id: object.id,
            before_image: Some(object),
            after_image: None,
        }
    }
}

/// Execution error types
#[derive(Debug, thiserror::Error)]
pub enum VMExecutionError {
    #[error("VM execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Invalid bytecode: {0}")]
    InvalidBytecode(String),
    
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,
    
    #[error("Instruction limit exceeded")]
    InstructionLimitExceeded,
    
    #[error("Timeout exceeded")]
    TimeoutExceeded,
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Controller validation failed: {0}")]
    ControllerValidationFailed(String),
}

/// Abstract interface for different VM types
pub trait VMExecutor: Send + Sync {
    /// Get the VM type this executor handles
    fn vm_type(&self) -> VMType;
    
    /// Load bytecode and execute with given context
    fn load_and_execute(
        &self,
        bytecode: &[u8],
        context: &ExecutionContext,
    ) -> Result<Vec<ObjectEffect>, VMExecutionError>;
}

/// Validate that controller can only modify objects it controls
pub fn validate_object_effects(
    effects: &[ObjectEffect], 
    controller_id: UnitsObjectId
) -> Result<(), VMExecutionError> {
    for effect in effects {
        // If the object state changed, verify controller owns it
        if effect.before_image != effect.after_image {
            if let Some(after_obj) = &effect.after_image {
                if after_obj.controller_id != controller_id {
                    return Err(VMExecutionError::ControllerValidationFailed(
                        "Controller cannot modify objects it doesn't control".into()
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use units_core::constants::TOKEN_CONTROLLER_ID;

    #[test]
    fn test_execution_context() {
        let instruction = Instruction::new(
            TOKEN_CONTROLLER_ID,
            "transfer".to_string(),
            vec![UnitsObjectId::new([1; 32])],
            vec![1, 2, 3, 4],
        );
        
        let mut objects = HashMap::new();
        let obj = UnitsObject::new_data(
            UnitsObjectId::new([1; 32]),
            TOKEN_CONTROLLER_ID,
            vec![100, 200],
        );
        objects.insert(obj.id, obj);
        
        let context = ExecutionContext::new(instruction, objects, 123, 456);
        
        assert_eq!(context.slot, 123);
        assert_eq!(context.timestamp, 456);
        assert_eq!(context.objects.len(), 1);
        assert_eq!(context.writable_objects().count(), 1);
    }

    #[test]
    fn test_object_effects() {
        let obj = UnitsObject::new_data(
            UnitsObjectId::new([1; 32]),
            TOKEN_CONTROLLER_ID,
            vec![1, 2, 3],
        );
        
        let creation = ObjectEffect::creation(obj.clone());
        assert!(creation.before_image.is_none());
        assert!(creation.after_image.is_some());
        
        let modified_obj = UnitsObject::new_data(
            UnitsObjectId::new([1; 32]),
            TOKEN_CONTROLLER_ID,
            vec![4, 5, 6],
        );
        
        let modification = ObjectEffect::modification(obj.clone(), modified_obj);
        assert!(modification.before_image.is_some());
        assert!(modification.after_image.is_some());
        
        let deletion = ObjectEffect::deletion(obj);
        assert!(deletion.before_image.is_some());
        assert!(deletion.after_image.is_none());
    }
}