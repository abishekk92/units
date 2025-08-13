# UNITS Storage Architecture Specification

## Overview

This document specifies the unified object architecture for UNITS storage, where **everything is an object** with immutable controllers that define mutation rules through sandboxed execution.

## Core Principles

1. **Unified Object Model**: All entities (data, code, accounts, tokens) are UnitsObjects
2. **Controller-Based Security**: Each object has an immutable controller (kernel module) 
3. **Sandboxed Execution**: Controllers run in isolated VM environments (RISC-V, WASM, etc.)
4. **Storage Simplicity**: Single key-value store (UnitsObjectId → UnitsObject)
5. **Extensible VMs**: Support for multiple VM types via pluggable executors

## Object Model

### Core Structure

```rust
/// VM types for executable objects
/// Currently only RISC-V is implemented - other types are reserved for future extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VMType {
    /// RISC-V ELF shared objects (currently implemented)
    RiscV,
    // Future VM types will be added here:
    // Wasm,   - WebAssembly modules (planned)
    // Ebpf,   - eBPF programs (planned) 
    // Native, - x86_64 native code (planned)
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

### System Constants

```rust
/// Hardcoded system controller IDs for bootstrap and security
/// Simple hardcoded values for initial implementation simplicity
pub const SYSTEM_LOADER_ID: UnitsObjectId = UnitsObjectId::new([0; 32]);
pub const TOKEN_CONTROLLER_ID: UnitsObjectId = UnitsObjectId::new([1; 32]);
pub const ACCOUNT_CONTROLLER_ID: UnitsObjectId = UnitsObjectId::new([2; 32]);
pub const MODULE_MANAGER_ID: UnitsObjectId = UnitsObjectId::new([3; 32]);
```

### Object Types by Usage

#### Kernel Modules (Controllers)
```rust
UnitsObject {
    id: controller_id,
    controller_id: SYSTEM_LOADER_ID,           // System loader controls kernel modules
    object_type: ObjectType::Executable(VMType::RiscV),
    data: risc_v_elf_bytes,                    // RISC-V ELF shared object
}
```

#### Data Objects  
```rust
UnitsObject {
    id: data_object_id,
    controller_id: TOKEN_CONTROLLER_ID,        // Token controller manages this data
    object_type: ObjectType::Data,
    data: token_balance_data,                  // Arbitrary binary data
}
```

## Execution Model

### VM Executor Interface

```rust
/// Abstract interface for different VM types
pub trait VMExecutor: Send + Sync {
    fn vm_type(&self) -> VMType;
    
    /// Load bytecode and execute with given context
    fn load_and_execute(
        &self,
        bytecode: &[u8],
        context: &ExecutionContext,
    ) -> Result<Vec<ObjectEffect>>;
}
```

### Execution Context

```rust
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
    
    
    // Note: env_vars field planned for future implementation
}

impl ExecutionContext {
    /// Get objects that this controller can modify (it controls)
    pub fn writable_objects(&self) -> impl Iterator<Item = (&UnitsObjectId, &UnitsObject)> {
        self.objects.iter().filter(|(_, obj)| {
            obj.controller_id == self.instruction.controller_id
        })
    }
    
    /// Get all objects (read-only + writable)
    pub fn all_objects(&self) -> &HashMap<UnitsObjectId, UnitsObject> {
        &self.objects
    }
}
```

### Controller Standard Interface

All kernel modules must implement a standard `main` entrypoint regardless of VM type:

**RISC-V Controllers (Current Rust Implementation):**
```rust
/// Current implementation uses pure Rust with units-kernel-sdk
/// Controllers implement the KernelModule trait for type-safe execution

use units_kernel_sdk::{KernelModule, ExecutionContext, ObjectEffect, KernelError};

pub struct TokenModule;

impl KernelModule for TokenModule {
    fn execute(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        // Dispatch based on target function
        match ctx.instruction.target_function.as_str() {
            "create_token" => Self::handle_create_token(ctx),
            "transfer_token" => Self::handle_transfer_token(ctx),
            "mint_token" => Self::handle_mint_token(ctx),
            "burn_token" => Self::handle_burn_token(ctx),
            _ => Err(KernelError::UnknownFunction),
        }
    }
}

// Safe memory management via SDK allocator
use_default_allocator!();

// Example function handlers
impl TokenModule {
    fn handle_transfer_token(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        // Implementation uses safe Rust - no unsafe code required
    }
}
```

**Legacy C Interface (Documentation Reference):**
```c
// Historical reference - current implementation uses Rust
#define INPUT_BUFFER_ADDR  0x10000000
#define OUTPUT_BUFFER_ADDR 0x20000000
#define MAX_BUFFER_SIZE    (1024 * 1024)  // 1MB limit

int main(void) {
    // Legacy C-based controller interface
    // Replaced by safe Rust implementation above
}
```

**Future VM Types:**
- **WASM**: Export `main()` function with same memory layout
- **eBPF**: Standard eBPF program structure with `main` section
- All VMs use consistent input/output buffer conventions

### Object Effects

Controllers return object state changes from instruction execution. In the current implementation, `ObjectEffect` and `TransactionEffect` are unified:

```rust
/// Effect of controller execution on a single object
/// Represents before/after state for one object in an instruction
/// Note: This is currently aliased as TransactionEffect for compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectEffect {
    /// The object that was modified
    pub object_id: UnitsObjectId,
    
    /// State before instruction execution (None if object was created)
    pub before_image: Option<UnitsObject>,
    
    /// State after instruction execution (None if object was deleted)  
    pub after_image: Option<UnitsObject>,
}

// Current implementation aliases for compatibility
pub type TransactionEffect = ObjectEffect;

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
```

### Effect Validation Rules

All ObjectEffects are validated before applying to storage:

1. **Ownership Validation**: Controller can only modify objects it controls
   - Effect object must have `controller_id` matching the executing controller
   
2. **ID Consistency**: Effect object_id must match target object
   - `effect.object_id == effect.object.id` for all effect types
   
3. **Type Preservation**: Controllers cannot arbitrarily change object_type
   - `object_type` changes require explicit business logic validation
   - System controllers have broader privileges for type changes
   
4. **Size Limits**: Objects have maximum size constraints
   - Default: 10MB per object data payload
   - Prevents resource exhaustion attacks

### Current Transaction Effect Implementation

In the current implementation, transaction effects are simplified to track single object changes:

```rust
/// Current unified effect structure for single object changes
/// Note: This represents the current implementation - future versions may
/// separate ObjectEffect and TransactionEffect for multi-object transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEffect {
    /// The transaction that caused this effect
    pub transaction_hash: TransactionHash,
    
    /// The single object that was modified
    pub object_id: UnitsObjectId,
    
    /// State before instruction execution (None if object was created)
    pub before_image: Option<UnitsObject>,
    
    /// State after instruction execution (None if object was deleted)  
    pub after_image: Option<UnitsObject>,
}
```

**Current Relationship**:
- **ObjectEffect**: Aliased to `TransactionEffect` in current implementation
- **TransactionEffect**: Single object's before/after state from one instruction
- **TransactionReceipt**: Contains `Vec<TransactionEffect>` for multi-object transactions

**Future Evolution**: The architecture reserves space for separating these concepts when implementing cross-controller communication and multi-object transactions.

## Transaction Execution Pipeline

### Flow Overview

```
1. Transaction with Instructions arrives
2. For each Instruction:
   a. Validate controller access to target objects
   b. Load target objects from storage into ExecutionContext
   c. Load controller kernel module from storage
   d. Determine VM type from controller.object_type
   e. Execute controller.target_function in appropriate VM sandbox
   f. Controller returns Vec<ObjectEffect> for objects it modified
   g. Validate ObjectEffects: controller can only modify objects it controls
   h. Collect all ObjectEffects from instruction
3. Create TransactionEffect containing all ObjectEffects from all instructions
4. Apply object changes to storage
5. Generate proofs for modified objects  
6. Return TransactionReceipt containing TransactionEffect and proofs
```

### Multi-Instruction Transactions

A single transaction can contain multiple instructions:
- Each instruction calls a specific function in a controller kernel module
- Instructions can target different controllers (but cross-controller calls reserved for future)
- All target objects from all instructions are validated and loaded
- Each controller can read any object but only modify objects it controls

### Instruction Format

```rust
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
```

### Controller Access Patterns

**Read Access**: Controllers can read any object (no restrictions)
**Write Access**: Controllers can only write to objects they control

```rust
/// Validate controller access to target objects
fn validate_controller_access(instruction: &Instruction, storage: &dyn Storage) -> Result<()> {
    let controller_id = instruction.controller_id;
    
    for object_id in &instruction.target_objects {
        let object = storage.get(object_id)?;
        
        // Controllers can read any object, but can only write objects they control
        if object.controller_id != controller_id {
            // This object will be read-only for this controller
            // Write attempts will be caught during effect validation
        }
    }
    
    Ok(())
}

/// Validate that controller can only modify objects it controls
fn validate_object_effects(effects: &[ObjectEffect], controller_id: UnitsObjectId) -> Result<()> {
    for effect in effects {
        // If the object state changed, verify controller owns it
        if effect.before_image != effect.after_image {
            if let Some(after_obj) = &effect.after_image {
                if after_obj.controller_id != controller_id {
                    return Err("Controller cannot modify objects it doesn't control".into());
                }
            }
        }
    }
    
    Ok(())
}
```

## System Bootstrap

### Bootstrap Process

```
1. System starts with hardcoded SYSTEM_LOADER_ID
2. System loader is self-controlling (controller_id = SYSTEM_LOADER_ID)  
3. System loader loads other kernel modules from storage
4. Kernel modules are controlled by system loader
5. Data objects are controlled by appropriate kernel modules
```

### System Loader Responsibilities

- Parse and validate ELF files
- Load controllers into VM sandboxes
- Orchestrate controller execution
- Apply object effects to storage
- Generate transaction receipts and proofs

## VM Implementations

### RISC-V Executor (Primary)

**Implementation**: Uses `ckb-vm` crate for production-ready, sandboxed RISC-V execution with ELF support.

```rust
pub struct RiscVExecutor {
    memory_limit: usize,        // 1MB default
    instruction_limit: u64,     // Configurable limit
}

impl VMExecutor for RiscVExecutor {
    fn vm_type(&self) -> VMType { VMType::RiscV }
    
    fn load_and_execute(&self, elf_bytes: &[u8], context: &ExecutionContext) -> Result<Vec<ObjectEffect>> {
        // 1. Initialize ckb-vm machine with memory limits
        // 2. Load ELF binary into VM memory space
        // 3. Set up fixed memory buffers (INPUT_BUFFER_ADDR, OUTPUT_BUFFER_ADDR)
        // 4. Serialize ExecutionContext and write to INPUT_BUFFER_ADDR
        // 5. Call main() entrypoint with instruction limits
        // 6. Read ObjectEffects from OUTPUT_BUFFER_ADDR and deserialize
        // 7. Return effects for validation and application
    }
}
```

### Future VM Extensions

The architecture supports adding new VM types without breaking changes:

```rust
pub struct WasmExecutor { /* WASM runtime */ }
pub struct EbpfExecutor { /* eBPF runtime */ }

// Register with system loader
system_loader.register_vm_executor(VMType::Wasm, Box::new(WasmExecutor::new()));
system_loader.register_vm_executor(VMType::Ebpf, Box::new(EbpfExecutor::new()));
```

## Security Model

### Immutable Controllers
- Object's `controller_id` is set at creation and cannot be changed
- Provides security guarantee: controller logic cannot be bypassed
- Controller changes require creating new object with new controller

### Sandboxed Execution
- Controllers run in isolated VM environments
- No direct storage access - only through provided object context
- Resource limits (memory, instructions, time) enforced per execution
- All mutations captured as structured effects for validation

### System Controller Whitelist
- Hardcoded system controller IDs prevent privilege escalation
- System loader controls creation of new kernel modules
- Clear chain of trust from bootstrap to all objects

## Storage Architecture

### Trait-Based Storage Design

The storage system follows a **composition over inheritance** pattern with clear separation:

```rust
// units-storage: Trait definitions only
use units_storage::{
    ObjectStorage,     // Core object persistence
    ProofStorage,      // Cryptographic proof management  
    ReceiptStorage,    // Transaction receipt tracking
    WriteAheadLog,     // Optional durability logging
    LockManager,       // Concurrency control
};

// units-storage-impl: Concrete implementations
use units_storage_impl::{
    InMemoryObjectStorage,
    ConsolidatedUnitsStorage,
    InMemoryReceiptStorage,
};
```

### Key-Value Mapping
```
UnitsObjectId (32 bytes) → UnitsObject (serialized)
```

### Object Proof Generation
- Each object mutation generates cryptographic proof
- Proofs commit to before/after object states  
- Slot-level aggregation of all object proofs
- Complete audit trail of all mutations

### Current Transaction Effect Storage
```rust
// Current implementation stores individual object effects
pub struct TransactionEffect {
    pub transaction_hash: TransactionHash,
    pub object_id: UnitsObjectId,
    pub before_image: Option<UnitsObject>, 
    pub after_image: Option<UnitsObject>,
}
```

## Migration Strategy

### Backward Compatibility
- New UnitsObject coexists with current ObjectType/ObjectMetadata
- Gradual migration of objects to new format
- RuntimeBackend system maintained alongside new VM executors
- Existing tests continue to pass during transition

### Implementation Phases
1. **Phase 1**: Unified UnitsObject struct and basic RISC-V execution
2. **Phase 2**: Complete transaction pipeline integration  
3. **Phase 3**: Attestation and advanced security features
4. **Phase 4**: Additional VM types (WASM, eBPF) and cross-controller communication

## Implementation Status

### ✅ Currently Implemented
- **RISC-V VM Execution**: Production-ready with rvsim integration
- **Unified Object Model**: Complete UnitsObject structure
- **Storage Traits**: Clean separation of concerns (units-storage vs units-storage-impl)
- **Kernel SDK**: Safe Rust development framework with allocator abstraction
- **Token Module**: Reference implementation in pure Rust
- **Transaction Processing**: ObjectEffect validation and application
- **Proof Generation**: Cryptographic object state commitments

### 🚧 Planned Extensions

#### Additional VM Types
- **WebAssembly**: WASM module execution (architecture defined, implementation pending)
- **eBPF**: eBPF program support (architecture defined, implementation pending)  
- **Native**: x86_64 native code execution (architecture defined, implementation pending)

#### Cross-Controller Communication
- Remove single-controller validation constraint
- Add controller dependency resolution
- Implement secure cross-controller call interface

#### Multi-Runtime Optimization  
- Runtime detection via bytecode headers
- JIT compilation for performance-critical controllers
- Native code generation for trusted system controllers

#### Distributed Execution
- Network attestation protocols
- Consensus integration for distributed object modifications
- Cross-node object effect synchronization

### Migration Notes
- Current implementation uses simplified TransactionEffect (single object per effect)
- Future versions will expand to multi-object transactions
- VM interface designed for extensibility - new VM types can be added without breaking changes

## References

- RISC-V Specification: https://riscv.org/specifications/
- ELF Format: https://refspecs.linuxfoundation.org/elf/elf.pdf
- WebAssembly: https://webassembly.org/
- eBPF: https://ebpf.io/