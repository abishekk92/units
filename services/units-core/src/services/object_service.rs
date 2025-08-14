//! Object service for high-level object operations
//!
//! This service provides convenient methods for working with UNITS objects,
//! including validation, transformation, and batch operations.

use std::sync::Arc;
use std::collections::HashMap;

use units_core_types::{
    UnitsObjectId, UnitsObject, ObjectType, VMType,
    TransactionHash, UnitsObjectProof,
};

use crate::error::{ServiceError, ServiceResult};
use super::storage_service::StorageService;

/// Object validator for enforcing business rules
pub struct ObjectValidator {
    /// Maximum object data size in bytes
    max_data_size: usize,
    /// Maximum controller program size
    max_program_size: usize,
}

impl ObjectValidator {
    pub fn new(max_data_size: usize, max_program_size: usize) -> Self {
        Self {
            max_data_size,
            max_program_size,
        }
    }

    /// Validate an object
    pub fn validate(&self, object: &UnitsObject) -> ServiceResult<()> {
        // Check data size
        if object.data().len() > self.max_data_size {
            return Err(ServiceError::invalid_request(
                format!("Object data size {} exceeds maximum {}", 
                    object.data().len(), self.max_data_size)
            ));
        }

        // Type-specific validation
        match &object.object_type {
            ObjectType::Executable(_vm_type) => {
                self.validate_program(object)?;
            }
            ObjectType::Data => {
                // Basic data objects have minimal validation
            }
        }

        // Validate controller - all objects have a controller
        let controller_id = object.controller_id();
        if controller_id == object.id() {
            return Err(ServiceError::invalid_request(
                "Object cannot control itself"
            ));
        }

        Ok(())
    }

    /// Validate program object
    fn validate_program(&self, object: &UnitsObject) -> ServiceResult<()> {
        if object.data().len() > self.max_program_size {
            return Err(ServiceError::invalid_request(
                format!("Program size {} exceeds maximum {}", 
                    object.data().len(), self.max_program_size)
            ));
        }

        // Ensure program has VM type
        if object.vm_type().is_none() {
            return Err(ServiceError::invalid_request(
                "Program object must specify VM type"
            ));
        }

        Ok(())
    }

    /// Validate account object
    fn validate_account(&self, _object: &UnitsObject) -> ServiceResult<()> {
        // Account-specific validation
        // Could check for required fields, permissions, etc.
        Ok(())
    }

    /// Validate token object
    fn validate_token(&self, _object: &UnitsObject) -> ServiceResult<()> {
        // Token-specific validation
        // Could check for valid amounts, metadata, etc.
        Ok(())
    }
}

/// Main object service providing high-level object operations
pub struct ObjectService {
    storage_service: Arc<StorageService>,
    validator: Arc<ObjectValidator>,
}

impl ObjectService {
    pub fn new(
        storage_service: Arc<StorageService>,
        max_data_size: usize,
        max_program_size: usize,
    ) -> Self {
        let validator = Arc::new(ObjectValidator::new(max_data_size, max_program_size));
        
        Self {
            storage_service,
            validator,
        }
    }

    /// Create a new object
    pub async fn create_object(
        &self,
        id: UnitsObjectId,
        object_type: ObjectType,
        data: Vec<u8>,
        controller: Option<UnitsObjectId>,
        _vm_type: Option<VMType>,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<(UnitsObject, UnitsObjectProof)> {
        // Check if object already exists
        if let Ok(_) = self.storage_service.objects().get_object(&id).await {
            return Err(ServiceError::invalid_request(
                format!("Object {} already exists", hex::encode(id.bytes()))
            ));
        }

        // Create object
        let controller_id = controller.unwrap_or(id); // Use self as controller if none specified
        let object = match object_type {
            ObjectType::Data => UnitsObject::new_data(id, controller_id, data),
            ObjectType::Executable(vm_type) => UnitsObject::new_executable(id, controller_id, vm_type, data),
        };

        // Validate
        self.validator.validate(&object)?;

        // Store
        let proof = self.storage_service.objects()
            .store_object(object.clone(), transaction_hash)
            .await?;

        Ok((object, proof))
    }

    /// Update an existing object
    pub async fn update_object(
        &self,
        id: &UnitsObjectId,
        data: Option<Vec<u8>>,
        controller: Option<Option<UnitsObjectId>>,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<(UnitsObject, UnitsObjectProof)> {
        // Load existing object
        let object = self.storage_service.objects().get_object(id).await?;

        // Apply updates - create new object with updated fields
        let updated_data = data.unwrap_or_else(|| object.data().to_vec());
        let updated_controller = controller.map_or(*object.controller_id(), |c| c.unwrap_or(*object.controller_id()));
        
        let object = match &object.object_type {
            ObjectType::Data => UnitsObject::new_data(*object.id(), updated_controller, updated_data),
            ObjectType::Executable(vm_type) => UnitsObject::new_executable(*object.id(), updated_controller, *vm_type, updated_data),
        };

        // Validate updated object
        self.validator.validate(&object)?;

        // Store
        let proof = self.storage_service.objects()
            .store_object(object.clone(), transaction_hash)
            .await?;

        Ok((object, proof))
    }

    /// Delete an object
    pub async fn delete_object(
        &self,
        id: &UnitsObjectId,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<UnitsObjectProof> {
        // Verify object exists
        let _ = self.storage_service.objects().get_object(id).await?;

        // Delete
        self.storage_service.objects()
            .delete_object(id, transaction_hash)
            .await
    }

    /// Get object with validation
    pub async fn get_object(&self, id: &UnitsObjectId) -> ServiceResult<UnitsObject> {
        let object = self.storage_service.objects().get_object(id).await?;
        
        // Validate on retrieval to ensure consistency
        self.validator.validate(&object)?;
        
        Ok(object)
    }

    /// Batch get objects
    pub async fn get_objects(
        &self,
        ids: &[UnitsObjectId],
    ) -> ServiceResult<HashMap<UnitsObjectId, UnitsObject>> {
        self.storage_service.objects().get_objects(ids).await
    }

    /// Get objects by controller
    pub async fn get_objects_by_controller(
        &self,
        controller_id: &UnitsObjectId,
        limit: usize,
    ) -> ServiceResult<Vec<UnitsObject>> {
        // This would require an index in storage
        // For now, return empty vector as placeholder
        let _ = (controller_id, limit);
        Ok(Vec::new())
    }

    /// Get objects by type
    pub async fn get_objects_by_type(
        &self,
        object_type: ObjectType,
        limit: usize,
    ) -> ServiceResult<Vec<UnitsObject>> {
        // This would require an index in storage
        // For now, return empty vector as placeholder
        let _ = (object_type, limit);
        Ok(Vec::new())
    }

    /// Transfer object control
    pub async fn transfer_control(
        &self,
        object_id: &UnitsObjectId,
        new_controller: Option<UnitsObjectId>,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<(UnitsObject, UnitsObjectProof)> {
        self.update_object(
            object_id,
            None,
            Some(new_controller),
            transaction_hash,
        ).await
    }

    /// Create a program object
    pub async fn create_program(
        &self,
        id: UnitsObjectId,
        bytecode: Vec<u8>,
        vm_type: VMType,
        controller: Option<UnitsObjectId>,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<(UnitsObject, UnitsObjectProof)> {
        self.create_object(
            id,
            ObjectType::Executable(vm_type),
            bytecode,
            controller,
            None, // vm_type is already in ObjectType
            transaction_hash,
        ).await
    }

    /// Create a token object
    pub async fn create_token(
        &self,
        id: UnitsObjectId,
        token_data: Vec<u8>,
        controller: Option<UnitsObjectId>,
        transaction_hash: Option<TransactionHash>,
    ) -> ServiceResult<(UnitsObject, UnitsObjectProof)> {
        self.create_object(
            id,
            ObjectType::Data, // Token is just a data object
            token_data,
            controller,
            None,
            transaction_hash,
        ).await
    }

    /// Get object statistics
    pub async fn get_stats(&self) -> ServiceResult<ObjectStats> {
        let cache_stats = self.storage_service.objects().cache_stats().await;
        
        Ok(ObjectStats {
            total_objects: 0, // Would need storage count method
            cache_size: cache_stats.size,
            cache_max_size: cache_stats.max_size,
            max_object_size: self.validator.max_data_size,
            max_program_size: self.validator.max_program_size,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ObjectStats {
    pub total_objects: u64,
    pub cache_size: usize,
    pub cache_max_size: usize,
    pub max_object_size: usize,
    pub max_program_size: usize,
}