//! Service layer modules for UNITS core functionality
//!
//! This module organizes business logic into focused service components
//! that coordinate between storage, runtime, and other system components.

pub mod transaction_service;
pub mod storage_service;
pub mod proof_service;
pub mod slot_service;
pub mod object_service;

// Re-export service types
pub use transaction_service::TransactionService;
pub use storage_service::StorageService;
pub use proof_service::ProofService;
pub use slot_service::SlotService;
pub use object_service::ObjectService;

// Service factory for dependency injection
pub mod factory;
// Commented out unused exports
// pub use factory::{ServiceFactory, ServiceContainer, ServiceStatus};

// Simple build test
pub mod simple_build_test;

// Minimal working services
pub mod minimal_services;
pub use minimal_services::{MinimalServiceFactory, MinimalServiceContainer};