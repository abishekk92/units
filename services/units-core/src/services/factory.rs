//! Service factory for dependency injection and service lifecycle management
//!
//! This module provides centralized service creation and configuration,
//! managing dependencies between different service components.

use std::sync::Arc;

use units_core_types::Runtime;
use units_storage_impl::ConsolidatedUnitsStorage;

use crate::config::Config;
use crate::error::ServiceResult;

use super::{
    TransactionService, StorageService, ProofService, SlotService, ObjectService,
    slot_service::SlotConfig,
};

/// Service container holding all initialized services
pub struct ServiceContainer {
    pub transaction_service: Arc<TransactionService>,
    pub storage_service: Arc<StorageService>,
    pub proof_service: Arc<ProofService>,
    pub slot_service: Arc<SlotService>,
    pub object_service: Arc<ObjectService>,
    pub runtime: Arc<dyn Runtime + Send + Sync>,
    pub storage: Arc<ConsolidatedUnitsStorage>,
}

/// Factory for creating and configuring services
pub struct ServiceFactory;

impl ServiceFactory {
    /// Create all services with dependencies properly injected
    pub fn create_services(
        config: Config,
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
    ) -> ServiceResult<ServiceContainer> {
        // Create storage service first as others depend on it
        let storage_service = Arc::new(StorageService::new(
            storage.clone(),
            config.storage.max_object_size / 1024, // Convert to KB for cache size
            300, // 5 minute cache TTL
        ));

        // Create proof service
        let proof_service = Arc::new(ProofService::new(
            storage.clone(),
            runtime.clone(),
        ));

        // Create transaction service
        let transaction_service = Arc::new(TransactionService::new(
            runtime.clone(),
            storage.clone(),
            config.server.max_connections as usize, // Use max connections as pool size
        ));

        // Create slot service
        let slot_config = SlotConfig {
            slot_duration_ms: 1000, // 1 second slots
            max_transactions_per_slot: 1000,
            auto_advance: true,
            grace_period_ms: 100,
        };

        let slot_service = Arc::new(SlotService::new(
            slot_config,
            transaction_service.clone(),
            proof_service.clone(),
        ));

        // Create object service
        let object_service = Arc::new(ObjectService::new(
            storage_service.clone(),
            config.storage.max_object_size,
            config.runtime.max_memory_bytes, // Use memory limit for program size
        ));

        Ok(ServiceContainer {
            transaction_service,
            storage_service,
            proof_service,
            slot_service,
            object_service,
            runtime,
            storage: storage,
        })
    }

    /// Create services with custom configuration
    pub fn create_with_options(
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
        options: ServiceOptions,
    ) -> ServiceResult<ServiceContainer> {
        // Create storage service
        let storage_service = Arc::new(StorageService::new(
            storage.clone(),
            options.cache_size,
            options.cache_ttl_secs,
        ));

        // Create proof service
        let proof_service = Arc::new(ProofService::new(
            storage.clone(),
            runtime.clone(),
        ));

        // Create transaction service
        let transaction_service = Arc::new(TransactionService::new(
            runtime.clone(),
            storage.clone(),
            options.transaction_pool_size,
        ));

        // Create slot service
        let slot_service = Arc::new(SlotService::new(
            options.slot_config,
            transaction_service.clone(),
            proof_service.clone(),
        ));

        // Create object service
        let object_service = Arc::new(ObjectService::new(
            storage_service.clone(),
            options.max_object_size,
            options.max_program_size,
        ));

        Ok(ServiceContainer {
            transaction_service,
            storage_service,
            proof_service,
            slot_service,
            object_service,
            runtime,
            storage,
        })
    }

    /// Create minimal services for testing
    pub fn create_test_services(
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
    ) -> ServiceResult<ServiceContainer> {
        let options = ServiceOptions::test_defaults();
        Self::create_with_options(runtime, storage, options)
    }
}

/// Options for customizing service creation
#[derive(Debug, Clone)]
pub struct ServiceOptions {
    /// Object cache size
    pub cache_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Transaction pool size
    pub transaction_pool_size: usize,
    /// Maximum object size
    pub max_object_size: usize,
    /// Maximum program size
    pub max_program_size: usize,
    /// Slot configuration
    pub slot_config: SlotConfig,
}

impl ServiceOptions {
    /// Create options from config
    pub fn from_config(config: &Config) -> Self {
        Self {
            cache_size: 1000,
            cache_ttl_secs: 300,
            transaction_pool_size: config.server.max_connections as usize,
            max_object_size: config.storage.max_object_size,
            max_program_size: config.runtime.max_memory_bytes,
            slot_config: SlotConfig::default(),
        }
    }

    /// Default options for testing
    pub fn test_defaults() -> Self {
        Self {
            cache_size: 100,
            cache_ttl_secs: 60,
            transaction_pool_size: 10,
            max_object_size: 1024 * 1024, // 1MB
            max_program_size: 512 * 1024,  // 512KB
            slot_config: SlotConfig {
                slot_duration_ms: 100,
                max_transactions_per_slot: 10,
                auto_advance: false,
                grace_period_ms: 10,
            },
        }
    }
}

impl ServiceContainer {
    /// Start all services
    pub async fn start(&self) -> ServiceResult<()> {
        // Start slot service
        self.slot_service.start().await?;
        
        // Additional service startup logic would go here
        
        Ok(())
    }

    /// Stop all services
    pub async fn stop(&self) -> ServiceResult<()> {
        // Graceful shutdown logic would go here
        // - Stop accepting new transactions
        // - Finalize current slot
        // - Flush caches
        // - Close connections
        
        Ok(())
    }

    /// Get service health status
    pub async fn health_check(&self) -> ServiceResult<HealthReport> {
        let slot_info = self.slot_service.slot_info().await;
        let storage_stats = self.storage_service.get_stats().await?;
        let proof_stats = self.proof_service.get_stats().await?;
        let tx_pool_stats = self.transaction_service.get_pool_stats().await;
        
        Ok(HealthReport {
            status: ServiceStatus::Healthy,
            current_slot: slot_info.current_slot,
            pending_transactions: tx_pool_stats.pending_count,
            cache_size: storage_stats.cache_stats.size,
            latest_proven_slot: proof_stats.latest_proven_slot,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub status: ServiceStatus,
    pub current_slot: units_core_types::SlotNumber,
    pub pending_transactions: usize,
    pub cache_size: usize,
    pub latest_proven_slot: units_core_types::SlotNumber,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}