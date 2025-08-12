# UNITS (Universal Information Tokenization System)

A modular storage and runtime system for the Universal Information Tokenization System (UNITS).

## Overview

UNITS is a component of Finternet that provides a way to tokenize and manage objects. The core of UNITS is the TokenizedObject, whose state can only be mutated by its holder. This workspace implements the full UNITS stack, organized into logical crates that work together.

## Workspace Structure

The project is organized as a Cargo workspace with the following crates:

- **units-core**: Core data structures and fundamental types
  - UnitsObjectId (32-byte public key)
  - TokenizedObject
  - Locks, transactions, and scheduling primitives
  - Proof systems and engines
  - Basic error types

- **units-storage-impl**: Storage backends and traits
  - Storage Traits
  - SQLite Implementation
  - Lock Manager
  - Key-value store where key is UnitsObjectId and value is TokenizedObject

- **units-runtime**: Runtime and verification
  - Object Runtime
  - Transaction Processing
  - Mock runtime for testing

## Features

- **TokenizedObject**: Core data structure controlled by holder's private key or controller program
- **Object Proofs**: Cryptographic proofs emitted whenever a value is set in storage, committing to previous and new states
- **Slot-Based Time**: Time split into configurable slots, with all object proofs from a slot aggregated into a slot proof
- **Storage Trait**: Unified interface for key-value storage backends
- **SQLite Backend**: Reliable and efficient storage implementation
- **Lock Manager**: Coordination for concurrent access to objects
- **Transaction Processing**: Support for complex multi-object operations

## Architecture

### Core Components

1. **UnitsObjectId**: 32-byte public key that uniquely identifies objects. The key is either controlled by its corresponding private key or by a `controller_program` for IDs not on the curve.

2. **TokenizedObject**: The fundamental data structure whose state can only be mutated by the holder.

3. **Storage**: Key-value store where the key is UnitsObjectId and the value is TokenizedObject as a blob.

### Proof System

The proof system provides cryptographic guarantees about object state changes:

1. **Object Proofs**: Emitted whenever a value is set in storage, these proofs commit to both the previous state and new state of the object.

2. **Slot Proofs**: Time is divided into slots (configurable length), and all object proofs from a slot are aggregated into a single slot proof that commits to the previous and new states of all objects in that slot.

### Storage Architecture

The storage system is built around a simple but powerful model:

- **Key-Value Interface**: Clean abstraction where UnitsObjectId maps to TokenizedObject
- **SQLite Backend**: Reliable persistence with ACID guarantees  
- **Lock Manager**: Coordinates concurrent access to prevent conflicts
- **Proof Integration**: Automatic generation of cryptographic proofs on state changes

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
# Use specific components:
units-core = "0.1.0"
units-storage-impl = "0.1.0"  # SQLite backend included
units-runtime = "0.1.0"
```

## Examples

### Basic Usage

```rust
use units_core::{UnitsObjectId, UnitsObject, TokenType};
use units_storage_impl::{SqliteStorage, UnitsStorage};
use std::path::Path;

// Create a storage instance
let storage = SqliteStorage::new(Path::new("./my_database.db")).await?;

// Create a token object
let id = UnitsObjectId::random();
let owner = UnitsObjectId::random();
let token_manager = UnitsObjectId::random();
let obj = UnitsObject::new_token(
    id, 
    owner, 
    TokenType::Native, 
    token_manager, 
    vec![1, 2, 3, 4]
);

// Store the object and get its proof
let proof = storage.set(&obj, None).await?;
println!("Object proof: {:?}", proof);

// Retrieve the object
if let Some(retrieved) = storage.get(&id).await? {
    println!("Retrieved object: {:?}", retrieved);
}

// Delete the object and get the deletion proof
let deletion_proof = storage.delete(&id, None).await?;
println!("Deletion proof: {:?}", deletion_proof);
```

### Transaction Processing

```rust
use units_core::{Transaction, Instruction, AccessIntent, RuntimeType, TransactionHash};
use units_runtime::Runtime;

// Create instructions for a transaction
let instruction = Instruction::wasm(
    vec![/* parameters */],
    vec![(object_id, AccessIntent::Write)],
    code_object_id
);

// Create a transaction
let transaction_hash = TransactionHash::new([0u8; 32]);
let mut transaction = Transaction::new(vec![instruction], transaction_hash);

// Acquire locks based on object intents
let lock_manager = storage.lock_manager();
let _locks = transaction.acquire_locks(&lock_manager).await?;

// Execute the transaction through runtime
let receipt = runtime.execute_transaction(&transaction, &storage).await?;

// Store the transaction receipt
storage.store_receipt(&receipt).await?;

println!("Transaction commitment level: {:?}", receipt.commitment_level);
```

### Scanning Objects

```rust
// Iterate over all objects
let mut iterator = storage.scan().await?;
while let Some(obj) = iterator.next().await {
    println!("Found object: {:?}", obj?);
}
```

### Working with Proofs

```rust
use units_core::SlotNumber;

// Generate a state proof for current slot
let state_proof = storage.generate_state_proof(None).await?;
println!("State proof: {:?}", state_proof);

// Get the current proof for a specific object
if let Some(obj_proof) = storage.get_proof(&id).await? {
    // Verify the proof
    if storage.verify_proof(&id, &obj_proof).await? {
        println!("Proof verified!");
    }
}

// Get an object's state at a specific historical slot
let historical_slot = SlotNumber(12345);
if let Some(historical_obj) = storage.get_at_slot(&id, historical_slot).await? {
    println!("Object at slot {}: {:?}", historical_slot, historical_obj);
}

// Get an object's proof at a specific historical slot
if let Some(historical_proof) = storage.get_proof_at_slot(&id, historical_slot).await? {
    println!("Proof at slot {}: {:?}", historical_slot, historical_proof);
}

// Iterate through transaction receipts
let mut receipts = storage.get_receipts().await?;
while let Some(receipt) = receipts.next().await {
    println!("Receipt: {:?}", receipt?);
}
```

## Development

### Build Commands

```bash
# Build all crates
fish -c "cargo workspaces exec -- cargo build"

# Check all crates
fish -c "cargo workspaces exec -- cargo check"

# Run all tests
fish -c "cargo workspaces exec -- cargo test"

# Format code
fish -c "cargo workspaces exec -- cargo fmt"

# Build with release optimizations
fish -c "cargo workspaces exec -- cargo build --release"
```

### Testing

```bash
# Run tests for a specific crate
fish -c "cd units-core && cargo test"

# Run a specific test
fish -c "cargo test test_name"
```

Standard workflow: Build → Check → Test

## License

MIT License. See [LICENSE](LICENSE) for details.
