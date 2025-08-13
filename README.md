# UNITS Storage - Universal Information Tokenization System

A modular storage and runtime system for the Universal Information Tokenization System (UNITS), a core component of Finternet.

## Overview

UNITS implements a unified object architecture where **everything is an object** with immutable controllers that define mutation rules through sandboxed execution. All entities (data, code, accounts, tokens) are represented as `UnitsObject`s with cryptographically verifiable state transitions.

**Core Architecture**: Objects → Controllers → Sandboxed VMs → Verified Effects → Storage → Proofs

## Workspace Structure

### Core Components

- **units-core** - Fundamental types and data structures
  - `UnitsObjectId` - 32-byte cryptographic object identifiers
  - `UnitsObject` - Unified object model with controller-based access control
  - `TransactionEffect`/`ObjectEffect` - State transition tracking
  - System constants (`SYSTEM_LOADER_ID`, `TOKEN_CONTROLLER_ID`)
  - Locking, scheduling, and proof generation primitives

- **units-storage** - Storage trait definitions (composition-based architecture)
  - `ObjectStorage` - Core object persistence interface
  - `ProofStorage` - Cryptographic proof management
  - `ReceiptStorage` - Transaction receipt tracking
  - `WriteAheadLog` - Optional durability logging
  - `LockManager` - Concurrency control

- **units-storage-impl** - Concrete storage implementations
  - `ConsolidatedUnitsStorage` - Primary storage implementation
  - `InMemoryObjectStorage` - Development and testing storage
  - File-based write-ahead logging
  - Composable storage architecture

- **units-runtime** - VM execution and transaction processing
  - RISC-V VM executor with sandboxed controller execution
  - Transaction effect validation and application
  - Receipt generation and proof management
  - Host environment for kernel modules

### Kernel Module Framework

- **units-kernel-sdk** - Safe development framework for kernel modules
  - **Zero unsafe code required** - `use_default_allocator!()` macro
  - Thread-safe bump allocator for VM environments
  - Core types and execution context management
  - Error handling and serialization utilities

- **units-kernel-modules** - Reference kernel module implementations
  - **token/** - Complete ERC-20 style token implementation in pure Rust
  - Demonstrates best practices for kernel module development
  - Uses SDK allocator (no custom unsafe code)

## Current Implementation Status

### ✅ Production Ready
- **RISC-V VM Execution** - Sandboxed controller execution with rvsim
- **Unified Object Model** - Complete UnitsObject architecture
- **Storage Architecture** - Trait-based composition design
- **Kernel SDK** - Safe Rust development framework
- **Token Module** - Reference implementation
- **Proof Generation** - Cryptographic state commitments

### 🚧 Architecture Defined, Implementation Pending
- **WebAssembly VM** - WASM module execution
- **eBPF VM** - eBPF program support
- **Cross-Controller Communication** - Multi-controller transactions
- **Distributed Execution** - Network consensus integration

## Quick Start

### Basic Storage Operations

```rust
use units_storage_impl::ConsolidatedUnitsStorage;
use units_core::{UnitsObjectId, UnitsObject, ObjectType};

// Create unified storage
let storage = ConsolidatedUnitsStorage::create();

// Create object with controller
let object_id = UnitsObjectId::new([1u8; 32]);
let controller_id = UnitsObjectId::new([2u8; 32]);
let object = UnitsObject {
    id: object_id,
    controller_id,
    object_type: ObjectType::Data,
    data: vec![1, 2, 3, 4],
};

// Store with automatic proof generation
let proof = storage.inner().objects.set(&object, None)?;
storage.inner().proofs.store_object_proof(&proof)?;

// Retrieve object
let retrieved = storage.inner().objects.get(&object_id)?;
```

### Transaction Processing with Effects

```rust
use units_core::transaction::{TransactionEffect, ObjectEffect};

// Controllers return effects describing their changes
let effect = TransactionEffect {
    transaction_hash: tx_hash,
    object_id: token_id,
    before_image: Some(old_token_state),
    after_image: Some(new_token_state),
};

// Runtime validates and applies effects
runtime.validate_effects(&[effect], controller_id)?;
runtime.apply_effects(&[effect])?;
```

### Kernel Module Development

```rust
// No unsafe code required!
#![no_std]
#![no_main]

use units_kernel_sdk::{
    use_default_allocator, KernelModule,
    ExecutionContext, ObjectEffect, KernelError
};

// One line - handles all memory management
use_default_allocator!();

pub struct MyModule;

impl KernelModule for MyModule {
    fn execute(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        match ctx.instruction.target_function.as_str() {
            "my_function" => {
                // Safe Rust implementation
                // SDK handles memory, serialization, system calls
                Ok(vec![/* effects */])
            }
            _ => Err(KernelError::UnknownFunction)
        }
    }
}
```

### Receipt and Historical Queries

```rust
use units_storage::ReceiptStorage;

// Query receipts by slot
let receipts = storage.inner().receipts.get_receipts_for_slot(slot_num)?;

// Query receipts affecting specific object
let object_receipts = storage.inner().receipts.get_receipts_for_object(
    &object_id, Some(start_slot), Some(end_slot)
)?;

// Historical object states (if supported by storage implementation)
let historical_object = storage.inner().objects.get_at_slot(&object_id, slot)?;
```

## Architecture Highlights

### Composition Over Inheritance
Storage traits are focused and composable:
```rust
// Import trait definitions
use units_storage::{ObjectStorage, ProofStorage, ReceiptStorage};
// Import implementations
use units_storage_impl::{InMemoryObjectStorage, ConsolidatedUnitsStorage};

// Mix and match implementations
let custom_storage = units_storage::UnitsStorage::new(
    MyObjectStorage::new(),
    MyProofStorage::new(),
    Some(MyWriteAheadLog::new())
);
```

### Security Through Sandboxing
- Controllers run in isolated VM environments (currently RISC-V)
- No direct storage access - only through provided object context
- Resource limits (memory, instructions) enforced per execution
- All mutations captured as structured effects for validation

### Cryptographic Auditability
- Every object mutation generates cryptographic proof
- Complete before/after state history
- Slot-level proof aggregation
- Verifiable audit trail of all changes

## Building and Testing

```bash
# Build all workspace crates
fish -c "cargo workspaces exec -- cargo build"

# Run tests across workspace
fish -c "cargo workspaces exec -- cargo test"

# Check specific crate
fish -c "cd units-kernel-sdk && cargo check"

# Format code
fish -c "cargo workspaces exec -- cargo fmt"
```
## License

MIT License. See [LICENSE](LICENSE) for details.
