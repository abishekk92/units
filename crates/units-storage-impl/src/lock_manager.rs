//! Lock Manager Implementation
//! 
//! Provides concrete implementations of the LockManager trait for object-level locking.

use units_storage::LockManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use units_core_types::error::StorageError;
use units_core_types::id::UnitsObjectId;

/// Simple lock guard implementation
pub struct SimpleLockGuard {
    _object_id: UnitsObjectId,
}

unsafe impl Send for SimpleLockGuard {}
unsafe impl Sync for SimpleLockGuard {}

/// Simple in-memory lock manager for testing and development
pub struct InMemoryLockManager {
    #[allow(dead_code)]
    locks: Arc<Mutex<HashMap<UnitsObjectId, Arc<Mutex<()>>>>>,
}

impl InMemoryLockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryLockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LockManager for InMemoryLockManager {
    type Guard<'a> = SimpleLockGuard where Self: 'a;
    
    fn lock(&self, id: &UnitsObjectId) -> Result<Self::Guard<'_>, StorageError> {
        // For now, return a simple guard without actual locking
        // A full implementation would manage actual mutex locks
        Ok(SimpleLockGuard { _object_id: *id })
    }
    
    fn try_lock(&self, id: &UnitsObjectId) -> Result<Option<Self::Guard<'_>>, StorageError> {
        // For now, always succeed
        Ok(Some(SimpleLockGuard { _object_id: *id }))
    }
    
    fn lock_many(&self, ids: &[UnitsObjectId]) -> Result<Vec<Self::Guard<'_>>, StorageError> {
        // For now, return guards for all requested IDs
        let guards = ids.iter()
            .map(|id| SimpleLockGuard { _object_id: *id })
            .collect();
        Ok(guards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_manager_basic() {
        let lock_manager = InMemoryLockManager::new();
        let object_id = UnitsObjectId::random();

        // Test basic locking
        let _guard = lock_manager.lock(&object_id).unwrap();
        
        // Test try_lock
        let _try_guard = lock_manager.try_lock(&object_id).unwrap();
        
        // Test multiple locks
        let ids = [object_id, UnitsObjectId::random()];
        let _guards = lock_manager.lock_many(&ids).unwrap();
        
        assert_eq!(_guards.len(), 2);
    }
}