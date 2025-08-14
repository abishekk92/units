//! Minimal service implementations that compile successfully
//!
//! These are simplified versions of the full services that demonstrate
//! the architecture without complex dependencies.

use std::sync::Arc;
use crate::error::ServiceResult;
use units_core_types::{
    UnitsObjectId, UnitsObject, ObjectStorage,
    TransactionHash, Transaction,
    SlotNumber, Runtime,
};
use units_storage_impl::ConsolidatedUnitsStorage;

/// Minimal transaction service
pub struct MinimalTransactionService {
    runtime: Arc<dyn Runtime + Send + Sync>,
    storage: Arc<ConsolidatedUnitsStorage>,
}

impl MinimalTransactionService {
    pub fn new(
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
    ) -> Self {
        Self { runtime, storage }
    }

    pub async fn submit_transaction(&self, transaction: Transaction) -> ServiceResult<TransactionHash> {
        // Simple implementation - just return the hash
        Ok(transaction.hash)
    }

    pub async fn get_transaction(&self, _hash: &TransactionHash) -> ServiceResult<Transaction> {
        Err(crate::error::ServiceError::invalid_request("Not implemented"))
    }
}

/// Minimal object service
pub struct MinimalObjectService {
    storage: Arc<ConsolidatedUnitsStorage>,
}

impl MinimalObjectService {
    pub fn new(storage: Arc<ConsolidatedUnitsStorage>) -> Self {
        Self { storage }
    }

    pub async fn get_object(&self, id: &UnitsObjectId) -> ServiceResult<UnitsObject> {
        use units_core_types::UnitsStorage;
        self.storage
            .objects()
            .get(id)
            .map_err(crate::error::ServiceError::Storage)?
            .ok_or_else(|| crate::error::ServiceError::object_not_found(hex::encode(id.bytes())))
    }

    pub async fn create_object(
        &self,
        id: UnitsObjectId,
        object_type: units_core_types::objects::ObjectType,
        data: Vec<u8>,
        controller: Option<UnitsObjectId>,
    ) -> ServiceResult<UnitsObject> {
        let controller_id = controller.unwrap_or(id);
        let object = match object_type {
            units_core_types::objects::ObjectType::Data => 
                units_core_types::objects::UnitsObject::new_data(id, controller_id, data),
            units_core_types::objects::ObjectType::Executable(vm_type) => 
                units_core_types::objects::UnitsObject::new_executable(id, controller_id, vm_type, data),
        };
        
        // Store in storage
        use units_core_types::UnitsStorage;
        let _proof = self.storage.objects().set(&object, None)
            .map_err(crate::error::ServiceError::Storage)?;
        
        Ok(object)
    }
}

/// Minimal slot service
pub struct MinimalSlotService {
    current_slot: std::sync::atomic::AtomicU64,
}

impl MinimalSlotService {
    pub fn new() -> Self {
        Self {
            current_slot: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn current_slot(&self) -> SlotNumber {
        self.current_slot.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn advance_slot(&self) -> ServiceResult<SlotNumber> {
        let new_slot = self.current_slot.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        Ok(new_slot)
    }
}

/// Minimal service container
pub struct MinimalServiceContainer {
    pub transaction_service: Arc<MinimalTransactionService>,
    pub object_service: Arc<MinimalObjectService>,
    pub slot_service: Arc<MinimalSlotService>,
    pub storage: Arc<ConsolidatedUnitsStorage>,
    pub runtime: Arc<dyn Runtime + Send + Sync>,
}

impl MinimalServiceContainer {
    pub fn new(
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
    ) -> Self {
        let transaction_service = Arc::new(MinimalTransactionService::new(runtime.clone(), storage.clone()));
        let object_service = Arc::new(MinimalObjectService::new(storage.clone()));
        let slot_service = Arc::new(MinimalSlotService::new());

        Self {
            transaction_service,
            object_service,
            slot_service,
            storage,
            runtime,
        }
    }

    pub async fn health_check(&self) -> ServiceResult<MinimalHealthReport> {
        Ok(MinimalHealthReport {
            status: "healthy".to_string(),
            current_slot: self.slot_service.current_slot(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MinimalHealthReport {
    pub status: String,
    pub current_slot: SlotNumber,
}

/// Minimal service factory
pub struct MinimalServiceFactory;

impl MinimalServiceFactory {
    pub fn create_minimal_services(
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
    ) -> ServiceResult<MinimalServiceContainer> {
        Ok(MinimalServiceContainer::new(runtime, storage))
    }
}