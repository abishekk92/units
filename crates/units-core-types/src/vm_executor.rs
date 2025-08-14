//! VM Executor trait and related types for the UNITS system
//!
//! This module provides the core VM execution interfaces and supporting data structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::id::UnitsObjectId;
use crate::objects::{UnitsObject, VMType};
use crate::transaction::Instruction;

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
    
    #[error("Unsupported VM type: {0}")]
    UnsupportedVMType(String),
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