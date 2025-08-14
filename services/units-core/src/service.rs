use std::sync::Arc;

use units_core_types::id::UnitsObjectId;
use units_core_types::objects::UnitsObject;
use units_core_types::transaction::{Transaction, TransactionReceipt, TransactionHash};
use units_core_types::{Runtime, SlotNumber, ObjectStorage};
use units_storage_impl::ConsolidatedUnitsStorage;

use crate::config::Config;
use crate::error::ServiceResult;
use crate::services::{MinimalServiceFactory, MinimalServiceContainer};

/// Core UNITS service that handles business logic
#[derive(Clone)]
pub struct UnitsService {
    services: Arc<MinimalServiceContainer>,
    config: Config,
}

impl UnitsService {
    pub fn new(
        storage: Arc<ConsolidatedUnitsStorage>,
        runtime: Arc<dyn Runtime + Send + Sync>,
        config: Config,
    ) -> Self {
        // Create all services using the minimal factory
        let services = MinimalServiceFactory::create_minimal_services(
            runtime,
            storage,
        ).expect("Failed to create services");
        
        Self {
            services: Arc::new(services),
            config,
        }
    }
    
    /// Start all services
    pub async fn start(&self) -> ServiceResult<()> {
        // Simple implementation - no-op for now
        Ok(())
    }

    /// Get object by ID
    pub async fn get_object(&self, object_id: &UnitsObjectId) -> ServiceResult<UnitsObject> {
        use units_core_types::UnitsStorage;
        self.services.storage
            .objects()
            .get(object_id)
            .map_err(crate::error::ServiceError::Storage)?
            .ok_or_else(|| crate::error::ServiceError::object_not_found(hex::encode(object_id.bytes())))
    }

    /// Submit transaction to the transaction pool
    pub async fn submit_transaction(&self, transaction: Transaction) -> ServiceResult<TransactionHash> {
        // Simple implementation - just return the hash
        Ok(transaction.hash)
    }

    /// Get transaction from pool
    pub async fn get_transaction(&self, _tx_hash: &TransactionHash) -> ServiceResult<Transaction> {
        Err(crate::error::ServiceError::invalid_request("Not implemented in simple version"))
    }

    /// Get transaction receipt
    pub async fn get_transaction_receipt(&self, _tx_hash: &TransactionHash) -> ServiceResult<TransactionReceipt> {
        Err(crate::error::ServiceError::invalid_request("Not implemented in simple version"))
    }

    /// Get current slot number
    pub async fn get_current_slot(&self) -> ServiceResult<SlotNumber> {
        Ok(0) // Simple implementation
    }

    /// Get service statistics
    pub async fn get_service_stats(&self) -> ServiceResult<ServiceStats> {
        Ok(ServiceStats {
            current_slot: 0,
            pending_transactions: 0,
            cached_objects: 0,
            latest_proven_slot: 0,
        })
    }

    /// Health check
    pub async fn health_check(&self) -> ServiceResult<HealthStatus> {
        let health = self.services.health_check().await?;
        
        Ok(HealthStatus {
            status: health.status,
            slot: 0,
            object_count: 0,
            pending_transactions: 0,
        })
    }
    
    /// Advance to next slot manually
    pub async fn advance_slot(&self) -> ServiceResult<SlotNumber> {
        Ok(1) // Simple implementation
    }
    
    /// Create a new object
    pub async fn create_object(
        &self,
        id: UnitsObjectId,
        object_type: units_core_types::objects::ObjectType,
        data: Vec<u8>,
        controller: Option<UnitsObjectId>,
        _vm_type: Option<units_core_types::objects::VMType>,
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
        let _proof = self.services.storage.objects().set(&object, None)
            .map_err(crate::error::ServiceError::Storage)?;
        
        Ok(object)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub slot: u64,
    pub object_count: u64,
    pub pending_transactions: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServiceStats {
    pub current_slot: SlotNumber,
    pub pending_transactions: u64,
    pub cached_objects: u64,
    pub latest_proven_slot: SlotNumber,
}