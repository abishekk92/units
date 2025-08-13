# UNITS (Universal Information Tokenization System)

A modular storage and runtime system for the Universal Information Tokenization System (UNITS).

## Overview

UNITS is a component of Finternet that provides a way to tokenize and manage objects. This workspace implements the full UNITS stack, organized into logical crates that work together.

**The core principle**: Objects can only be mutated by their controller through sandboxed execution, with all changes cryptographically verified and auditable.

## Workspace Structure

The project is organized as a Cargo workspace with the following crates:

### Core Components

- **units-core**: Core data structures and fundamental types
  - `UnitsObjectId` - 32-byte object identifiers  
  - `UnitsObject` - Immutable objects with controller-based access control
  - Transaction types (`TransactionEffect`, `ObjectEffect`)
  - Locking primitives and scheduling
  - Cryptographic proof systems (Merkle Proofs, State Proofs)
  - Basic error types

- **units-storage-impl**: **Consolidated Storage Architecture** 
  - 🆕 **Modern trait-based design** with clear separation of concerns
  - `ObjectStorage` - Core object persistence
  - `ProofStorage` - Cryptographic proof management  
  - `ReceiptStorage` - Transaction receipt tracking
  - `WriteAheadLog` - Optional durability logging
  - `LockManager` - Concurrency control
  - In-memory implementations for testing/development
  - ⚠️  Legacy SQLite backend (deprecated in favor of new architecture)

- **units-runtime**: Runtime and VM execution
  - VM execution environment for kernel modules
  - `ObjectEffect` validation and application
  - Transaction processing and receipt generation
  - Host environment for sandboxed controllers

### Kernel Module Framework

- **units-kernel-sdk**: Framework for building kernel modules
  - 🆕 **Safe allocator abstraction** - no unsafe code required for kernel authors
  - Core types (`UnitsObjectId`, `ExecutionContext`, `ObjectEffect`)  
  - System call interface for sandboxed execution
  - Built-in error handling and serialization

- **units-kernel-modules/token**: Example token implementation
  - Complete ERC-20 style token functionality
  - Uses safe SDK allocator (no custom unsafe code)
  - Demonstrates best practices for kernel module development

## Architecture

### Consolidated Storage Design

The new storage architecture follows **composition over inheritance**:

```rust
// Modern approach - compose storage capabilities
use units_storage_impl::{
    ObjectStorage, ProofStorage, WriteAheadLog, ReceiptStorage,
    ConsolidatedUnitsStorage
};

// Create storage with exactly the capabilities you need
let storage = ConsolidatedUnitsStorage::new();

// Or compose your own
let custom_storage = UnitsStorage::new(
    MyObjectStorage::new(),
    MyProofStorage::new(), 
    Some(MyWriteAheadLog::new())
);
```

**Benefits:**
- **55% reduction** in trait complexity (from ~1,800 lines to ~800 lines)
- **Clear separation of concerns** - each trait has a single responsibility
- **Easy testing and mocking** - focused interfaces
- **Better performance** - no complex inheritance hierarchies

### Object Effects: The Heart of UNITS

Objects in UNITS are **immutable** and can only be modified through `ObjectEffect`s:

```rust
pub struct ObjectEffect {
    pub object_id: UnitsObjectId,
    pub before_image: Option<UnitsObject>,  // None = creation
    pub after_image: Option<UnitsObject>,   // None = deletion
}
```

**Why ObjectEffect exists:**
1. **Sandboxed Controllers**: Kernel modules run in isolated VMs and can't directly modify storage
2. **Security Validation**: System validates that controllers only modify objects they own
3. **Audit Trail**: Complete before/after history for cryptographic proofs
4. **Cross-VM Portability**: Uniform interface across RISC-V, WASM, eBPF execution

**Execution Flow:**
```
Controller VM → ObjectEffects → Validation → Storage → Proofs
```

### Kernel Module Development

The SDK provides everything needed for safe kernel module development:

```rust
// In your kernel module's main.rs
#![no_std]
#![no_main]

use units_kernel_sdk::use_default_allocator;

// One line - no unsafe code needed!
use_default_allocator!();

// Your kernel module logic...
```

The SDK handles:
- ✅ **Memory allocation** - safe, thread-safe bump allocator
- ✅ **System calls** - I/O, context reading, effect writing  
- ✅ **Error handling** - standardized error types
- ✅ **Serialization** - Borsh-based object encoding

## Usage

### Basic Storage Operations

```rust
use units_storage_impl::ConsolidatedUnitsStorage;
use units_core::{UnitsObjectId, UnitsObject};

// Create consolidated storage instance
let storage = ConsolidatedUnitsStorage::new();

// Create and store an object  
let id = UnitsObjectId::new([1u8; 32]);
let controller = UnitsObjectId::new([2u8; 32]);
let object = UnitsObject::new(id, controller, vec![1, 2, 3]);

// Store with proof generation
let proof = storage.objects.set(&object, None)?;
storage.proofs.store_object_proof(&proof)?;

// Retrieve object
if let Some(retrieved) = storage.objects.get(&id)? {
    println!("Found object: {:?}", retrieved);
}
```

### Transaction Processing with ObjectEffects

```rust
use units_core::transaction::ObjectEffect;

// Controllers return ObjectEffects describing their changes
let effects = vec![
    ObjectEffect::creation(new_token),
    ObjectEffect::modification(old_balance, new_balance),
];

// Runtime validates and applies effects
runtime.validate_effects(&effects, controller_id)?;
runtime.apply_effects(&effects)?;
```

### Receipt Storage

```rust
use units_storage_impl::ReceiptStorage;

// Store transaction receipt
storage.receipts.store_receipt(&receipt)?;

// Query receipts by slot
let slot_receipts = storage.receipts.get_receipts_for_slot(12345)?;

// Query receipts affecting specific object
let object_receipts = storage.receipts.get_receipts_for_object(
    &object_id, Some(start_slot), Some(end_slot)
)?;
```

### Historical Queries

```rust
use units_storage_impl::HistoricalStorage;

// Get object at specific slot
let historical_object = storage.objects.get_at_slot(&id, slot_num)?;

// Get object history over time range
let history = storage.objects.get_history(&id, start_slot, end_slot)?;

// Get proof history
let proof_history = storage.proofs.get_proof_history(
    &id, Some(start_slot), Some(end_slot)
)?;
```

## Migration from Legacy Code

The codebase maintains **backward compatibility** while encouraging migration:

```rust
// ⚠️ Legacy (deprecated)
use units_storage_impl::LegacyUnitsStorage;

// ✅ Modern (recommended)  
use units_storage_impl::{ObjectStorage, ProofStorage, ConsolidatedUnitsStorage};
```

All legacy traits are marked `#[deprecated]` with migration guidance.

## Key Improvements

### Storage Architecture
- **Trait consolidation**: From 8 overlapping traits to 5 focused traits
- **Composition pattern**: Flexible capability combinations
- **Standard iterators**: No complex async adapters required
- **Clear responsibilities**: Object storage ≠ proof storage ≠ WAL

### Kernel Module Framework  
- **Zero unsafe code**: `use_default_allocator!()` macro handles everything
- **Thread-safe allocator**: Atomic operations for VM safety
- **Consistent interface**: Same allocator for all kernel modules
- **Easy development**: Focus on business logic, not memory management

### Developer Experience
- **Focused traits**: Single responsibility interfaces
- **Better documentation**: Clear examples and migration paths  
- **Reduced complexity**: 55% fewer lines in storage layer
- **Modern patterns**: Composition over inheritance throughout

## Building

```bash
# Build all crates
cargo build

# Run tests  
cargo test

# Check a specific crate
cd units-kernel-sdk && cargo check
```

## License

MIT License. See [LICENSE](LICENSE) for details.