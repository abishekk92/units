//! Consolidated Storage Implementation
//! 
//! This module provides a working implementation of the consolidated storage
//! architecture without depending on the complex legacy SQLite implementation.

use units_storage::{ObjectStorage, HistoricalStorage, ProofStorage, WriteAheadLog};
use std::collections::HashMap;
use std::sync::RwLock;
use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::objects::UnitsObject;
use units_core::proofs::{SlotNumber, StateProof, UnitsObjectProof};

/// Simple in-memory object storage implementation
pub struct InMemoryObjectStorage {
    objects: RwLock<HashMap<UnitsObjectId, UnitsObject>>,
    history: RwLock<HashMap<(UnitsObjectId, SlotNumber), UnitsObject>>,
}

impl InMemoryObjectStorage {
    pub fn new() -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
            history: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryObjectStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectStorage for InMemoryObjectStorage {
    fn get(&self, id: &UnitsObjectId) -> Result<Option<UnitsObject>, StorageError> {
        let objects = self.objects.read().unwrap();
        Ok(objects.get(id).cloned())
    }
    
    fn set(
        &self,
        object: &UnitsObject,
        _transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError> {
        let mut objects = self.objects.write().unwrap();
        objects.insert(*object.id(), object.clone());
        
        // Create a simple proof (this would be more complex in reality)
        Ok(UnitsObjectProof {
            object_id: *object.id(),
            slot: 0, // Would be current slot
            object_hash: [0u8; 32], // Would be actual object hash
            prev_proof_hash: None,
            transaction_hash: _transaction_hash,
            proof_data: vec![], // Would be actual proof data
        })
    }
    
    fn delete(
        &self,
        id: &UnitsObjectId,
        _transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError> {
        let mut objects = self.objects.write().unwrap();
        objects.remove(id);
        
        // Create a simple proof for deletion
        Ok(UnitsObjectProof {
            object_id: *id,
            slot: 0,
            object_hash: [0u8; 32], // Would be actual object hash
            prev_proof_hash: None,
            transaction_hash: _transaction_hash,
            proof_data: vec![],
        })
    }
    
    fn iter(&self) -> Box<dyn Iterator<Item = Result<UnitsObject, StorageError>> + '_> {
        let objects = self.objects.read().unwrap();
        let objects_vec: Vec<_> = objects.values().cloned().collect();
        Box::new(objects_vec.into_iter().map(Ok))
    }
}

impl HistoricalStorage for InMemoryObjectStorage {
    fn get_at_slot(
        &self,
        id: &UnitsObjectId,
        slot: SlotNumber,
    ) -> Result<Option<UnitsObject>, StorageError> {
        let history = self.history.read().unwrap();
        Ok(history.get(&(*id, slot)).cloned())
    }
    
    fn get_history(
        &self,
        id: &UnitsObjectId,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<(SlotNumber, UnitsObject)>, StorageError> {
        let history = self.history.read().unwrap();
        Ok(history
            .iter()
            .filter(|((obj_id, slot), _)| {
                *obj_id == *id && *slot >= start_slot && *slot <= end_slot
            })
            .map(|((_, slot), obj)| (*slot, obj.clone()))
            .collect())
    }
    
    fn compact_history(&self, _before_slot: SlotNumber) -> Result<usize, StorageError> {
        // Simple implementation - could compact history here
        Ok(0)
    }
}

/// Simple in-memory proof storage
pub struct InMemoryProofStorage {
    object_proofs: RwLock<HashMap<UnitsObjectId, Vec<(SlotNumber, UnitsObjectProof)>>>,
    state_proofs: RwLock<HashMap<SlotNumber, StateProof>>,
}

impl InMemoryProofStorage {
    pub fn new() -> Self {
        Self {
            object_proofs: RwLock::new(HashMap::new()),
            state_proofs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryProofStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofStorage for InMemoryProofStorage {
    fn store_object_proof(&self, proof: &UnitsObjectProof) -> Result<(), StorageError> {
        let mut proofs = self.object_proofs.write().unwrap();
        proofs
            .entry(proof.object_id)
            .or_insert_with(Vec::new)
            .push((proof.slot, proof.clone()));
        Ok(())
    }
    
    fn get_latest_proof(&self, id: &UnitsObjectId) -> Result<Option<UnitsObjectProof>, StorageError> {
        let proofs = self.object_proofs.read().unwrap();
        Ok(proofs
            .get(id)
            .and_then(|proofs| proofs.iter().max_by_key(|(slot, _)| slot))
            .map(|(_, proof)| proof.clone()))
    }
    
    fn get_proof_history(
        &self,
        id: &UnitsObjectId,
        start_slot: Option<SlotNumber>,
        end_slot: Option<SlotNumber>,
    ) -> Result<Vec<(SlotNumber, UnitsObjectProof)>, StorageError> {
        let proofs = self.object_proofs.read().unwrap();
        Ok(proofs
            .get(id)
            .unwrap_or(&Vec::new())
            .iter()
            .filter(|(slot, _)| {
                if let Some(start) = start_slot {
                    if *slot < start { return false; }
                }
                if let Some(end) = end_slot {
                    if *slot > end { return false; }
                }
                true
            })
            .cloned()
            .collect())
    }
    
    fn store_state_proof(&self, proof: &StateProof) -> Result<(), StorageError> {
        let mut proofs = self.state_proofs.write().unwrap();
        proofs.insert(proof.slot, proof.clone());
        Ok(())
    }
    
    fn get_state_proof(&self, slot: SlotNumber) -> Result<Option<StateProof>, StorageError> {
        let proofs = self.state_proofs.read().unwrap();
        Ok(proofs.get(&slot).cloned())
    }
    
    fn get_state_proof_history(
        &self,
        start_slot: SlotNumber,
        end_slot: SlotNumber,
    ) -> Result<Vec<StateProof>, StorageError> {
        let proofs = self.state_proofs.read().unwrap();
        Ok(proofs
            .values()
            .filter(|proof| proof.slot >= start_slot && proof.slot <= end_slot)
            .cloned()
            .collect())
    }
}

// Re-export from lock_manager module
pub use crate::lock_manager::{InMemoryLockManager, SimpleLockGuard};

/// No-op write-ahead log implementation
pub struct NoOpWriteAheadLog;

impl WriteAheadLog for NoOpWriteAheadLog {
    fn record_update(
        &self,
        _object: &UnitsObject,
        _proof: &UnitsObjectProof,
        _transaction_hash: Option<[u8; 32]>,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    
    fn record_state_proof(&self, _state_proof: &StateProof) -> Result<(), StorageError> {
        Ok(())
    }
    
    fn replay<F>(&self, _callback: F) -> Result<(), StorageError>
    where
        F: FnMut(&UnitsObject, &UnitsObjectProof) -> Result<(), StorageError>,
    {
        Ok(())
    }
}

/// Complete consolidated storage implementation using composition
pub struct ConsolidatedUnitsStorage {
    inner: units_storage::UnitsStorage<InMemoryObjectStorage, InMemoryProofStorage, NoOpWriteAheadLog>,
}

impl ConsolidatedUnitsStorage {
    pub fn create() -> Self {
        Self {
            inner: units_storage::UnitsStorage::new(
                InMemoryObjectStorage::new(),
                InMemoryProofStorage::new(),
                Some(NoOpWriteAheadLog),
            )
        }
    }
    
    /// Get access to the inner storage
    pub fn inner(&self) -> &units_storage::UnitsStorage<InMemoryObjectStorage, InMemoryProofStorage, NoOpWriteAheadLog> {
        &self.inner
    }
    
    /// Get mutable access to the inner storage
    pub fn inner_mut(&mut self) -> &mut units_storage::UnitsStorage<InMemoryObjectStorage, InMemoryProofStorage, NoOpWriteAheadLog> {
        &mut self.inner
    }
}

impl Default for ConsolidatedUnitsStorage {
    fn default() -> Self {
        Self::create()
    }
}