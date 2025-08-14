//! Simple build test to verify service architecture compiles
//!
//! This is a simplified version of the services with minimal functionality
//! to ensure the core architecture compiles correctly.

use std::sync::Arc;
use crate::error::ServiceResult;
use crate::config::Config;
use units_core_types::Runtime;
use units_storage_impl::ConsolidatedUnitsStorage;

pub struct SimpleServiceContainer {
    pub runtime: Arc<dyn Runtime + Send + Sync>,
    pub storage: Arc<ConsolidatedUnitsStorage>,
}

pub struct SimpleServiceFactory;

impl SimpleServiceFactory {
    pub fn create_simple_services(
        _config: Config,
        runtime: Arc<dyn Runtime + Send + Sync>,
        storage: Arc<ConsolidatedUnitsStorage>,
    ) -> ServiceResult<SimpleServiceContainer> {
        Ok(SimpleServiceContainer {
            runtime,
            storage,
        })
    }
}

impl SimpleServiceContainer {
    pub async fn health_check(&self) -> ServiceResult<SimpleHealthReport> {
        Ok(SimpleHealthReport {
            status: "healthy".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SimpleHealthReport {
    pub status: String,
}