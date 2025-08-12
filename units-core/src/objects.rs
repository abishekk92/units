use crate::id::UnitsObjectId;
use serde::{Deserialize, Serialize};

/// VM types for executable objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VMType {
    /// RISC-V ELF shared objects (primary implementation)
    RiscV,
}

/// Object type distinguishing data from executable objects
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    /// Data object - not executable
    Data,
    /// Executable object with specific VM type
    Executable(VMType),
}

/// Unified object structure for all UNITS entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnitsObject {
    /// Unique identifier - how object is indexed in storage
    pub id: UnitsObjectId,

    /// Immutable controller - defines mutation rules for this object
    /// Points to another UnitsObject with ObjectType::Executable
    pub controller_id: UnitsObjectId,

    /// Object type - data or executable with VM specification
    pub object_type: ObjectType,

    /// Object payload: ELF/WASM/eBPF bytecode or arbitrary data
    pub data: Vec<u8>,
}

impl UnitsObject {
    /// Create a new data object
    pub fn new_data(id: UnitsObjectId, controller_id: UnitsObjectId, data: Vec<u8>) -> Self {
        Self {
            id,
            controller_id,
            object_type: ObjectType::Data,
            data,
        }
    }

    /// Create a new executable object (kernel module)
    pub fn new_executable(
        id: UnitsObjectId,
        controller_id: UnitsObjectId,
        vm_type: VMType,
        bytecode: Vec<u8>,
    ) -> Self {
        Self {
            id,
            controller_id,
            object_type: ObjectType::Executable(vm_type),
            data: bytecode,
        }
    }

    /// Get the object ID
    pub fn id(&self) -> &UnitsObjectId {
        &self.id
    }

    /// Get the controller ID
    pub fn controller_id(&self) -> &UnitsObjectId {
        &self.controller_id
    }

    /// Get the object data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Check if this is a data object
    pub fn is_data(&self) -> bool {
        matches!(self.object_type, ObjectType::Data)
    }

    /// Check if this is an executable object
    pub fn is_executable(&self) -> bool {
        matches!(self.object_type, ObjectType::Executable(_))
    }

    /// Get VM type if this is an executable object
    pub fn vm_type(&self) -> Option<VMType> {
        match &self.object_type {
            ObjectType::Executable(vm_type) => Some(*vm_type),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_object() {
        // Create IDs for testing
        let id = UnitsObjectId::new([1; 32]);
        let controller_id = UnitsObjectId::new([2; 32]);
        let data = vec![0, 1, 2, 3, 4];

        // Create a data object
        let data_obj = UnitsObject::new_data(id, controller_id, data.clone());

        // Check type and accessors
        assert!(data_obj.is_data());
        assert!(!data_obj.is_executable());
        assert_eq!(data_obj.vm_type(), None);
        assert_eq!(data_obj.data(), &data);
        assert_eq!(data_obj.id(), &id);
        assert_eq!(data_obj.controller_id(), &controller_id);
    }

    #[test]
    fn test_executable_object() {
        // Create IDs for testing
        let id = UnitsObjectId::new([1; 32]);
        let controller_id = UnitsObjectId::new([2; 32]);
        let bytecode = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic bytes

        // Create an executable object
        let exec_obj =
            UnitsObject::new_executable(id, controller_id, VMType::RiscV, bytecode.clone());

        // Check type and accessors
        assert!(!exec_obj.is_data());
        assert!(exec_obj.is_executable());
        assert_eq!(exec_obj.vm_type(), Some(VMType::RiscV));
        assert_eq!(exec_obj.data(), &bytecode);
        assert_eq!(exec_obj.id(), &id);
        assert_eq!(exec_obj.controller_id(), &controller_id);
    }

    #[test]
    fn test_vm_types() {
        // Test RISC-V VM type (only supported type)
        let id = UnitsObjectId::new([1; 32]);
        let controller_id = UnitsObjectId::new([2; 32]);
        let bytecode = vec![1, 2, 3, 4];

        let riscv_obj = UnitsObject::new_executable(id, controller_id, VMType::RiscV, bytecode);
        assert_eq!(riscv_obj.vm_type(), Some(VMType::RiscV));
        assert!(riscv_obj.is_executable());
        assert!(!riscv_obj.is_data());
    }
}
