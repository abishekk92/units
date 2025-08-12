# Units Storage Simplification Guide

This guide outlines the major simplifications made to the units-storage codebase and how to migrate existing code.

## Overview of Changes

### 1. **Simplified Storage Traits** (`storage.rs`)
- **Old**: Complex `UnitsStorage` trait extending multiple traits with mixed concerns
- **New**: Focused traits with single responsibilities:
  - `ObjectStorage` - Basic object CRUD operations
  - `HistoricalStorage` - Time-travel capabilities
  - `ProofStorage` - Proof management
  - `WriteAheadLog` - Optional durability layer
  - `LockManager` - Simplified RAII-based locking

### 2. **Transaction Management Consolidation** (`transaction_manager.rs`)
- **Old**: Transaction operations split between Storage and Runtime
- **New**: All transaction operations in `TransactionManager` trait within Runtime
- Benefits: Single source of truth, clearer separation of concerns

### 3. **Simplified Iterators** (`iterators.rs`)
- **Old**: Complex async-to-sync adapters with `UnitsIterator`, `AsyncSource`, etc.
- **New**: Simple `StorageIterator<T>` wrapper with standard Rust patterns
- Benefits: Easier to understand, less overhead, better performance

### 4. **Concrete Proof Engine** (`proof_engine.rs`)
- **Old**: `ProofEngine` trait with only one implementation
- **New**: Concrete `ProofEngine` class
- Benefits: Less abstraction, easier to optimize, clearer code

### 5. **Unified Receipt Storage** (`receipt_storage.rs`)
- **Old**: `TransactionReceiptStorage` trait + receipt methods in `UnitsStorage`
- **New**: Single `ReceiptStorage` trait
- Benefits: No duplication, consistent interface

### 6. **Optional WAL**
- **Old**: WAL mandatory as part of `UnitsStorage`
- **New**: WAL is optional and composed separately
- Benefits: Flexibility for different storage backends

## Migration Examples

### Storage Implementation

**Old way:**
```rust
impl UnitsStorage for MyStorage {
    fn get(&self, id: &UnitsObjectId) -> Result<Option<UnitsObject>, StorageError> { ... }
    fn set(&self, object: &UnitsObject, tx_hash: Option<[u8; 32]>) -> Result<UnitsObjectProof, StorageError> { ... }
    fn execute_transaction_batch(...) -> Result<TransactionReceipt, StorageError> { ... }
    fn get_transaction_receipt(...) -> Result<Option<TransactionReceipt>, StorageError> { ... }
    // ... many more mixed methods
}
```

**New way:**
```rust
// Implement focused traits
impl ObjectStorage for MyStorage {
    fn get(&self, id: &UnitsObjectId) -> Result<Option<UnitsObject>, StorageError> { ... }
    fn set(&self, object: &UnitsObject, tx_hash: Option<[u8; 32]>) -> Result<UnitsObjectProof, StorageError> { ... }
    fn iter(&self) -> Box<dyn Iterator<Item = Result<UnitsObject, StorageError>> + '_> { ... }
}

impl ProofStorage for MyProofStore {
    fn store_object_proof(&self, proof: &UnitsObjectProof) -> Result<(), StorageError> { ... }
    fn get_latest_proof(&self, id: &UnitsObjectId) -> Result<Option<UnitsObjectProof>, StorageError> { ... }
}

// Transaction management now in Runtime
impl TransactionManager for MyRuntime {
    fn execute_transaction(&self, tx: &Transaction) -> Result<TransactionReceipt, RuntimeError> { ... }
    fn get_receipt(&self, hash: &TransactionHash) -> Result<Option<TransactionReceipt>, StorageError> { ... }
}
```

### Using Iterators

**Old way:**
```rust
let iter: Box<dyn UnitsStorageIterator> = storage.scan();
let adapter = AsyncSourceAdapter::new(source, rt);
// Complex async handling...
```

**New way:**
```rust
let objects: Vec<UnitsObject> = storage.iter()
    .filter_storage(|obj| obj.is_active())
    .collect_storage()?;

// Or with batching
let batches = storage.iter()
    .batch(100)
    .collect_storage()?;
```

### Proof Generation

**Old way:**
```rust
let proof_engine: &dyn ProofEngine = storage.proof_engine();
let proof = proof_engine.generate_object_proof(&object, prev_proof, tx_hash)?;
```

**New way:**
```rust
let proof_engine = ProofEngine::new();
let proof = proof_engine.generate_object_proof(&object, prev_proof, tx_hash)?;
```

### Composing Storage

**New way to compose storage components:**
```rust
use units_storage_impl::storage::{UnitsStorage, ObjectStorage, ProofStorage, WriteAheadLog};

// Create components
let object_store = SqliteObjectStorage::new(pool.clone());
let proof_store = SqliteProofStorage::new(pool.clone());
let wal = FileWriteAheadLog::new(wal_path)?;

// Compose them
let storage = UnitsStorage::new(
    object_store,
    proof_store,
    Some(wal), // WAL is optional
);

// Use the composed storage
let proof = storage.store_with_proof(&object, Some(tx_hash))?;
```

## Benefits Summary

1. **Clearer Separation of Concerns**: Each trait has a single, well-defined purpose
2. **Better Testability**: Smaller interfaces are easier to mock and test
3. **Flexibility**: Components can be mixed and matched as needed
4. **Performance**: Less abstraction overhead, especially in iterators
5. **Maintainability**: Simpler code is easier to understand and modify
6. **Type Safety**: Concrete types where abstraction isn't needed

## Gradual Migration Strategy

1. Start by implementing the new traits alongside the old ones
2. Move transaction operations to Runtime/TransactionManager
3. Replace iterator usage with the new simplified iterators
4. Switch to concrete ProofEngine
5. Consolidate receipt storage
6. Finally, remove old trait implementations

The old traits are still available for backward compatibility, allowing for gradual migration.