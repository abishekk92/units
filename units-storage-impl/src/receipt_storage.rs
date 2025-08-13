//! Unified Receipt Storage
//! 
//! This module consolidates transaction receipt storage into a single, focused trait.

use std::collections::HashMap;
use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::transaction::TransactionReceipt;
use units_core::proofs::SlotNumber;

/// Unified trait for transaction receipt storage
/// 
/// This replaces the fragmented receipt storage in the legacy traits
pub trait ReceiptStorage: Send + Sync {
    /// Store a transaction receipt
    fn store_receipt(
        &self,
        receipt: &TransactionReceipt,
    ) -> Result<(), StorageError>;
    
    /// Get a receipt by transaction hash
    fn get_receipt(
        &self,
        tx_hash: &[u8; 32],
    ) -> Result<Option<TransactionReceipt>, StorageError>;
    
    /// Get receipts for a specific slot
    fn get_receipts_for_slot(
        &self,
        slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Get receipts within a slot range
    fn get_receipts_range(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Get receipts affecting a specific object
    fn get_receipts_for_object(
        &self,
        object_id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<TransactionReceipt>, StorageError>;
    
    /// Delete old receipts before a slot (for cleanup)
    fn cleanup_receipts_before(
        &self,
        slot: SlotNumber,
    ) -> Result<usize, StorageError>;
}

/// Simple in-memory receipt storage for testing
pub struct InMemoryReceiptStorage {
    receipts: std::sync::RwLock<HashMap<[u8; 32], TransactionReceipt>>,
}

impl InMemoryReceiptStorage {
    pub fn new() -> Self {
        Self {
            receipts: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryReceiptStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiptStorage for InMemoryReceiptStorage {
    fn store_receipt(&self, receipt: &TransactionReceipt) -> Result<(), StorageError> {
        let mut receipts = self.receipts.write().unwrap();
        receipts.insert(receipt.transaction_hash, receipt.clone());
        Ok(())
    }
    
    fn get_receipt(&self, tx_hash: &[u8; 32]) -> Result<Option<TransactionReceipt>, StorageError> {
        let receipts = self.receipts.read().unwrap();
        Ok(receipts.get(tx_hash).cloned())
    }
    
    fn get_receipts_for_slot(&self, slot: SlotNumber) -> Result<Vec<TransactionReceipt>, StorageError> {
        let receipts = self.receipts.read().unwrap();
        Ok(receipts
            .values()
            .filter(|r| r.slot == slot)
            .cloned()
            .collect())
    }
    
    fn get_receipts_range(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        let receipts = self.receipts.read().unwrap();
        Ok(receipts
            .values()
            .filter(|r| r.slot >= start_slot && r.slot <= end_slot)
            .cloned()
            .collect())
    }
    
    fn get_receipts_for_object(
        &self,
        object_id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<TransactionReceipt>, StorageError> {
        let receipts = self.receipts.read().unwrap();
        Ok(receipts
            .values()
            .filter(|r| {
                // Check slot range
                if let Some(start) = start_slot {
                    if r.slot < start { return false; }
                }
                if let Some(end) = end_slot {
                    if r.slot > end { return false; }
                }
                
                // Check if this receipt affects the object
                // Note: This is a simplified check - would need proper transaction effect parsing
                r.object_proofs.contains_key(object_id)
            })
            .cloned()
            .collect())
    }
    
    fn cleanup_receipts_before(&self, slot: SlotNumber) -> Result<usize, StorageError> {
        let mut receipts = self.receipts.write().unwrap();
        let initial_len = receipts.len();
        receipts.retain(|_, receipt| receipt.slot >= slot);
        Ok(initial_len - receipts.len())
    }
}