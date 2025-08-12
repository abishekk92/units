use std::collections::HashMap;
use std::sync::Mutex;

use units_core::error::{RuntimeError, StorageError};
use units_core::id::UnitsObjectId;
use units_core::objects::{UnitsObject, VMType};
use units_core::transaction::{
    ConflictResult, Transaction, TransactionHash, TransactionReceipt,
};
use units_core::proofs::SlotNumber;

use crate::runtime::Runtime;
use crate::vm_executor::VMExecutor;
use crate::riscv_executor::RiscVExecutor;
use units_storage_impl::storage_traits::{TransactionReceiptStorage, UnitsReceiptIterator};

/// Mock implementation of the Runtime trait for testing purposes
pub struct MockRuntime {
    /// Store of transactions by their hash
    transactions: HashMap<TransactionHash, Transaction>,
    /// Store of transaction receipts by transaction hash
    receipts: HashMap<TransactionHash, TransactionReceipt>,
    /// Current slot for transaction processing
    current_slot: SlotNumber,
    /// Mock objects in memory (used for testing)
    objects: HashMap<UnitsObjectId, UnitsObject>,
}

impl MockRuntime {
    /// Create a new MockRuntime
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            receipts: HashMap::new(),
            current_slot: 0,
            objects: HashMap::new(),
        }
    }

    /// Add a transaction to the mock runtime's transaction store
    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.transactions.insert(transaction.hash, transaction);
    }

    /// Add a receipt to the mock runtime
    pub fn add_receipt(&mut self, receipt: TransactionReceipt) {
        self.receipts.insert(receipt.transaction_hash, receipt);
    }

    /// Get the current slot
    pub fn current_slot(&self) -> SlotNumber {
        self.current_slot
    }

    /// Set the current slot
    pub fn set_current_slot(&mut self, slot: SlotNumber) {
        self.current_slot = slot;
    }

    /// Add an object to mock storage
    pub fn add_object(&mut self, object: UnitsObject) {
        self.objects.insert(*object.id(), object);
    }

    /// Get objects for testing
    pub fn objects(&self) -> &HashMap<UnitsObjectId, UnitsObject> {
        &self.objects
    }
}

impl Runtime for MockRuntime {
    fn get_vm_executor(&self, vm_type: VMType) -> Option<Box<dyn VMExecutor>> {
        match vm_type {
            VMType::RiscV => Some(Box::new(RiscVExecutor::new())),
            _ => None, // Only RISC-V supported in mock for now
        }
    }

    fn execute_transaction(&self, _transaction: Transaction) -> TransactionReceipt {
        // Mock implementation - just return a basic receipt
        TransactionReceipt::new([0u8; 32], self.current_slot, true, 0)
    }

    fn check_conflicts(&self, _transaction: &Transaction) -> Result<ConflictResult, RuntimeError> {
        // Mock implementation - assume no conflicts
        Ok(ConflictResult::NoConflict)
    }

    fn get_transaction(&self, hash: &TransactionHash) -> Option<Transaction> {
        self.transactions.get(hash).cloned()
    }

    fn get_transaction_receipt(&self, hash: &TransactionHash) -> Option<TransactionReceipt> {
        self.receipts.get(hash).cloned()
    }

    fn rollback_transaction(&self, _transaction_hash: &TransactionHash) -> Result<bool, RuntimeError> {
        // Mock implementation - always succeed
        Ok(true)
    }
}

impl Default for MockRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockRuntime {
    fn clone(&self) -> Self {
        Self {
            transactions: self.transactions.clone(),
            receipts: self.receipts.clone(),
            current_slot: self.current_slot,
            objects: self.objects.clone(),
        }
    }
}

/// Simple in-memory receipt storage for testing
pub struct InMemoryReceiptStorage {
    receipts: Mutex<HashMap<TransactionHash, TransactionReceipt>>,
}

impl InMemoryReceiptStorage {
    pub fn new() -> Self {
        Self {
            receipts: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryReceiptStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionReceiptStorage for InMemoryReceiptStorage {
    fn store_receipt(&self, receipt: &TransactionReceipt) -> Result<(), StorageError> {
        self.receipts
            .lock()
            .map_err(|e| StorageError::LockError(format!("Lock error: {}", e)))?
            .insert(receipt.transaction_hash, receipt.clone());
        Ok(())
    }

    fn get_receipt(&self, hash: &TransactionHash) -> Result<Option<TransactionReceipt>, StorageError> {
        Ok(self.receipts
            .lock()
            .map_err(|e| StorageError::LockError(format!("Lock error: {}", e)))?
            .get(hash)
            .cloned())
    }

    fn get_receipts_for_object(&self, _id: &UnitsObjectId) -> Box<dyn UnitsReceiptIterator + '_> {
        // Simple implementation - return empty iterator
        let empty_receipts = vec![];
        Box::new(InMemoryReceiptIterator::new(empty_receipts))
    }

    fn get_receipts_in_slot(&self, slot: SlotNumber) -> Box<dyn UnitsReceiptIterator + '_> {
        let receipts: Vec<TransactionReceipt> = self.receipts
            .lock()
            .unwrap_or_else(|_| panic!("Lock poisoned"))
            .values()
            .filter(|receipt| receipt.slot == slot)
            .cloned()
            .collect();
        
        Box::new(InMemoryReceiptIterator::new(receipts))
    }
}

/// In-memory receipt iterator for testing
pub struct InMemoryReceiptIterator {
    receipts: std::vec::IntoIter<TransactionReceipt>,
}

impl InMemoryReceiptIterator {
    fn new(receipts: Vec<TransactionReceipt>) -> Self {
        Self {
            receipts: receipts.into_iter(),
        }
    }
}

impl Iterator for InMemoryReceiptIterator {
    type Item = Result<TransactionReceipt, StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.receipts.next().map(Ok)
    }
}

impl UnitsReceiptIterator for InMemoryReceiptIterator {}