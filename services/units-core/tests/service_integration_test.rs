//! Integration tests for UNITS services
//!
//! These tests demonstrate the minimal service layer functionality
//! that compiles and works with the current implementation.

use std::sync::Arc;

use units_core_types::{
    UnitsObjectId, Transaction, Instruction, CommitmentLevel,
};
use units_core_types::objects::{ObjectType, VMType};
use units_storage_impl::ConsolidatedUnitsStorage;
use units_runtime_impl::MockRuntime;

use units_core_service::services::{MinimalServiceFactory, MinimalServiceContainer};
use units_core_service::config::Config;
use units_core_service::service::UnitsService;

#[tokio::test]
async fn test_minimal_service_creation() {
    // Create test runtime and storage
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    
    // Create services using minimal factory
    let services = MinimalServiceFactory::create_minimal_services(
        runtime.clone(),
        storage.clone(),
    ).expect("Failed to create minimal services");
    
    // Verify services are created
    assert_eq!(services.slot_service.current_slot(), 0);
    
    // Test health check
    let health = services.health_check().await.expect("Health check failed");
    assert_eq!(health.status, "healthy");
    assert_eq!(health.current_slot, 0);
}

#[tokio::test]
async fn test_units_service_creation() {
    // Create test runtime and storage
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    let config = Config::default();
    
    // Create UnitsService
    let service = UnitsService::new(storage.clone(), runtime.clone(), config);
    
    // Test health check
    let health = service.health_check().await.expect("Health check failed");
    assert_eq!(health.status, "healthy");
    assert_eq!(health.slot, 0);
    assert_eq!(health.object_count, 0);
    assert_eq!(health.pending_transactions, 0);
}

#[tokio::test]
async fn test_object_operations() {
    // Setup
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    let config = Config::default();
    let service = UnitsService::new(storage.clone(), runtime.clone(), config);
    
    // Create an object
    let object_id = UnitsObjectId::new([1u8; 32]);
    let data = b"test object data".to_vec();
    
    let object = service.create_object(
        object_id,
        ObjectType::Data,
        data.clone(),
        None,
        None,
    ).await.expect("Failed to create object");
    
    assert_eq!(object.id(), &object_id);
    assert_eq!(object.data(), &data);
    
    // Retrieve the object
    let retrieved = service.get_object(&object_id).await.expect("Failed to get object");
    assert_eq!(retrieved.id(), &object_id);
    assert_eq!(retrieved.data(), &data);
}

#[tokio::test]
async fn test_transaction_operations() {
    // Setup
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    let config = Config::default();
    let service = UnitsService::new(storage.clone(), runtime.clone(), config);
    
    // Create a simple transaction
    let controller_id = UnitsObjectId::new([10u8; 32]);
    let target_id = UnitsObjectId::new([20u8; 32]);
    
    let instruction = Instruction {
        controller_id,
        target_function: "transfer".to_string(),
        target_objects: vec![target_id],
        params: b"instruction params".to_vec(),
    };
    
    let transaction = Transaction {
        hash: [99u8; 32],
        instructions: vec![instruction],
        commitment_level: CommitmentLevel::Committed,
    };
    
    // Submit transaction - this should work with minimal implementation
    let tx_hash = service.submit_transaction(transaction).await.expect("Failed to submit transaction");
    assert_eq!(tx_hash, [99u8; 32]);
    
    // Try to get transaction (should fail in minimal implementation)
    let result = service.get_transaction(&tx_hash).await;
    assert!(result.is_err()); // Expected to fail in minimal implementation
}

#[tokio::test]
async fn test_slot_operations() {
    // Setup
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    let config = Config::default();
    let service = UnitsService::new(storage.clone(), runtime.clone(), config);
    
    // Check initial slot
    let current_slot = service.get_current_slot().await.expect("Failed to get current slot");
    assert_eq!(current_slot, 0);
    
    // Try to advance slot (minimal implementation returns fixed value)
    let new_slot = service.advance_slot().await.expect("Failed to advance slot");
    assert_eq!(new_slot, 1); // Minimal implementation returns 1
}

#[tokio::test]
async fn test_storage_integration() {
    // Setup with direct storage access
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    let services = MinimalServiceFactory::create_minimal_services(
        runtime.clone(),
        storage.clone(),
    ).expect("Failed to create minimal services");
    
    // Create multiple objects through the service
    let object_ids: Vec<UnitsObjectId> = (0..3).map(|i| UnitsObjectId::new([i as u8; 32])).collect();
    
    for (i, &id) in object_ids.iter().enumerate() {
        let result = services.object_service.create_object(
            id,
            ObjectType::Data,
            format!("data {}", i).into_bytes(),
            None,
        ).await;
        
        assert!(result.is_ok(), "Failed to create object {}: {:?}", i, result.err());
    }
    
    // Retrieve all objects
    for (i, &id) in object_ids.iter().enumerate() {
        let obj = services.object_service.get_object(&id).await.expect("Failed to get object");
        assert_eq!(obj.data(), &format!("data {}", i).into_bytes());
    }
}

#[tokio::test]
async fn test_service_stats() {
    // Setup
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    let config = Config::default();
    let service = UnitsService::new(storage.clone(), runtime.clone(), config);
    
    // Get service stats
    let stats = service.get_service_stats().await.expect("Failed to get service stats");
    assert_eq!(stats.current_slot, 0);
    assert_eq!(stats.pending_transactions, 0);
    assert_eq!(stats.cached_objects, 0);
    assert_eq!(stats.latest_proven_slot, 0);
}

#[tokio::test] 
async fn test_executable_object_creation() {
    // Setup
    let runtime = Arc::new(MockRuntime::new());
    let storage = Arc::new(ConsolidatedUnitsStorage::new_in_memory());
    let config = Config::default();
    let service = UnitsService::new(storage.clone(), runtime.clone(), config);
    
    // Create an executable object
    let program_id = UnitsObjectId::new([42u8; 32]);
    let bytecode = b"mock bytecode".to_vec();
    
    let program_object = service.create_object(
        program_id,
        ObjectType::Executable(VMType::RiscV),
        bytecode.clone(),
        None,
        Some(VMType::RiscV),
    ).await.expect("Failed to create program object");
    
    assert_eq!(program_object.id(), &program_id);
    assert_eq!(program_object.data(), &bytecode);
    
    match program_object.object_type {
        ObjectType::Executable(vm_type) => assert_eq!(vm_type, VMType::RiscV),
        _ => panic!("Expected executable object type"),
    }
}