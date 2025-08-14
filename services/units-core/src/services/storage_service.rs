//! Storage service for managing object persistence
//!
//! This service provides high-level abstractions over the storage layer,
//! handling object lifecycle, caching, and coordination with proof generation.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

use units_core_types::{
    UnitsStorage, ObjectStorage, ProofStorage, HistoricalStorage,
    UnitsObjectId, UnitsObject, UnitsObjectProof,
    SlotNumber, StateProof, TransactionHash,
};
use units_storage_impl::ConsolidatedUnitsStorage;

use crate::error::{ServiceError, ServiceResult};

/// Object cache entry
#[derive(Clone)]
struct CacheEntry {
    object: UnitsObject,
    last_access: std::time::Instant,
}

/// Object manager with caching and validation
pub struct ObjectManager {
    storage: Arc<ConsolidatedUnitsStorage>,
    cache: Arc<RwLock<HashMap<UnitsObjectId, CacheEntry>>>,
    cache_size: usize,
    cache_ttl: std::time::Duration,
}

impl ObjectManager {
    pub fn new(
        storage: Arc<ConsolidatedUnitsStorage>,
        cache_size: usize,
        cache_ttl_secs: u64,
    ) -> Self {
        Self {
            storage,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_size,
            cache_ttl: std::time::Duration::from_secs(cache_ttl_secs),
        }
    }

    /// Get object with caching
    pub async fn get_object(&self, id: &UnitsObjectId) -> ServiceResult<UnitsObject> {
        // Check cache first
        if let Some(object) = self.get_from_cache(id).await {
            return Ok(object);
        }

        // Load from storage
        let object = self.storage
            .objects()
            .get(id)
            .map_err(ServiceError::Storage)?
            .ok_or_else(|| ServiceError::object_not_found(hex::encode(id.bytes())))?;

        // Add to cache
        self.add_to_cache(id, object.clone()).await;

        Ok(object)
    }

    /// Store object with proof generation
    pub async fn store_object(
        &self,
        object: UnitsObject,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<UnitsObjectProof> {
        // Validate object
        self.validate_object(&object)?;

        // Store in persistent storage
        let proof = self.storage
            .objects()
            .set(&object, transaction_hash)
            .map_err(ServiceError::Storage)?;

        // Update cache
        self.add_to_cache(object.id(), object.clone()).await;

        Ok(proof)
    }

    /// Delete object
    pub async fn delete_object(
        &self,
        id: &UnitsObjectId,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<UnitsObjectProof> {
        // Remove from cache
        self.remove_from_cache(id).await;

        // Delete from storage
        let proof = self.storage
            .objects()
            .delete(id, transaction_hash)
            .map_err(ServiceError::Storage)?;

        Ok(proof)
    }

    /// Batch get objects
    pub async fn get_objects(&self, ids: &[UnitsObjectId]) -> ServiceResult<HashMap<UnitsObjectId, UnitsObject>> {
        let mut objects = HashMap::new();
        
        for id in ids {
            match self.get_object(id).await {
                Ok(obj) => {
                    objects.insert(*id, obj);
                }
                Err(ServiceError::ObjectNotFound { .. }) => {
                    // Skip missing objects
                }
                Err(e) => return Err(e),
            }
        }

        Ok(objects)
    }

    /// Get object at specific slot (historical query)
    pub async fn get_object_at_slot(
        &self,
        id: &UnitsObjectId,
        slot: SlotNumber,
    ) -> ServiceResult<Option<UnitsObject>> {
        self.storage
            .historical()
            .get_at_slot(id, slot)
            .map_err(ServiceError::Storage)
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            size: cache.len(),
            max_size: self.cache_size,
            ttl_secs: self.cache_ttl.as_secs(),
        }
    }

    // Cache management helpers
    async fn get_from_cache(&self, id: &UnitsObjectId) -> Option<UnitsObject> {
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(id) {
            if entry.last_access.elapsed() < self.cache_ttl {
                entry.last_access = std::time::Instant::now();
                return Some(entry.object.clone());
            } else {
                // Entry expired
                cache.remove(id);
            }
        }
        
        None
    }

    async fn add_to_cache(&self, id: &UnitsObjectId, object: UnitsObject) {
        let mut cache = self.cache.write().await;
        
        // Evict oldest entry if cache is full
        if cache.len() >= self.cache_size {
            if let Some(oldest_id) = cache.iter()
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(id, _)| *id) {
                cache.remove(&oldest_id);
            }
        }
        
        cache.insert(*id, CacheEntry {
            object,
            last_access: std::time::Instant::now(),
        });
    }

    async fn remove_from_cache(&self, id: &UnitsObjectId) {
        self.cache.write().await.remove(id);
    }

    fn validate_object(&self, object: &UnitsObject) -> ServiceResult<()> {
        // Validate object size
        let serialized = bincode::serialize(object)
            .map_err(|e| ServiceError::Internal(e.into()))?;
        
        const MAX_OBJECT_SIZE: usize = 10 * 1024 * 1024; // 10MB
        if serialized.len() > MAX_OBJECT_SIZE {
            return Err(ServiceError::invalid_request(
                format!("Object size {} exceeds maximum {}", serialized.len(), MAX_OBJECT_SIZE)
            ));
        }

        // Additional validation based on object type
        match &object.object_type {
            units_core_types::objects::ObjectType::Executable(_vm_type) => {
                // Program validation - check bytecode size, etc.
            }
            units_core_types::objects::ObjectType::Data => {
                // Data object validation
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub ttl_secs: u64,
}

/// Main storage service that provides unified access to all storage operations
pub struct StorageService {
    storage: Arc<ConsolidatedUnitsStorage>,
    object_manager: Arc<ObjectManager>,
}

impl StorageService {
    pub fn new(
        storage: Arc<ConsolidatedUnitsStorage>,
        cache_size: usize,
        cache_ttl_secs: u64,
    ) -> Self {
        let object_manager = Arc::new(ObjectManager::new(
            storage.clone(),
            cache_size,
            cache_ttl_secs,
        ));

        Self {
            storage,
            object_manager,
        }
    }

    /// Get object manager
    pub fn objects(&self) -> &Arc<ObjectManager> {
        &self.object_manager
    }

    /// Get proof for a specific slot
    pub async fn get_slot_proof(&self, slot: SlotNumber) -> ServiceResult<Option<StateProof>> {
        self.storage
            .proofs()
            .get_state_proof(slot)
            .map_err(ServiceError::Storage)
    }

    /// Get object proof
    pub async fn get_object_proof(
        &self,
        object_id: &UnitsObjectId,
        _slot: SlotNumber,
    ) -> ServiceResult<Option<UnitsObjectProof>> {
        self.storage
            .proofs()
            .get_latest_proof(object_id)
            .map_err(ServiceError::Storage)
    }

    /// Store state proof
    pub async fn store_state_proof(&self, proof: StateProof) -> ServiceResult<()> {
        self.storage
            .proofs()
            .store_state_proof(&proof)
            .map_err(ServiceError::Storage)
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> ServiceResult<StorageStats> {
        let cache_stats = self.object_manager.cache_stats().await;
        
        // Get latest slot from proofs (simplified for build)
        let latest_slot = 0;

        Ok(StorageStats {
            cache_stats,
            latest_slot,
            object_count: 0, // Would need to add count method to storage
        })
    }

    /// Perform storage maintenance (cleanup, compaction, etc.)
    pub async fn maintenance(&self) -> ServiceResult<MaintenanceReport> {
        let start = std::time::Instant::now();
        
        // Clear expired cache entries
        self.object_manager.clear_cache().await;
        
        // Additional maintenance tasks would go here
        // - WAL cleanup
        // - Old proof cleanup
        // - Storage compaction
        
        Ok(MaintenanceReport {
            duration_ms: start.elapsed().as_millis() as u64,
            cache_cleared: true,
            proofs_cleaned: 0,
            objects_compacted: 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub cache_stats: CacheStats,
    pub latest_slot: SlotNumber,
    pub object_count: u64,
}

#[derive(Debug, Clone)]
pub struct MaintenanceReport {
    pub duration_ms: u64,
    pub cache_cleared: bool,
    pub proofs_cleaned: u64,
    pub objects_compacted: u64,
}