#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec, string::String, collections::BTreeMap, boxed::Box};

#[cfg(feature = "std")]
use std::{vec, vec::Vec, string::String, collections::HashMap, boxed::Box};

#[cfg(not(feature = "std"))]
units_kernel_sdk::use_default_allocator!();

use crate::{
    EnhancedAccountData, FlexCreateAccountParams, FlexUpdateAccountParams, 
    FlexAddRecoveryAddressParams, FlexRemoveRecoveryAddressParams, 
    FlexDeactivateAccountParams, FlexReactivateAccountParams, GetAccountParams,
    validate_username,
    auth::{
        AuthManager, AuthContext, AuthResult, AuthError,
        signature_schemes::{create_default_signature_authenticators},
        multi_factor::{create_default_mfa_authenticators},
        policies::{StandardAccountPolicy, HighSecurityPolicy}
    }
};
use units_kernel_sdk::{
    ExecutionContext, ObjectEffect, KernelModule, KernelError,
    UnitsObject, ObjectType, UnitsObjectId,
};

#[cfg(not(feature = "std"))]
#[allow(dead_code)]
type MetadataMap = BTreeMap<String, String>;

#[cfg(feature = "std")]
#[allow(dead_code)]
type MetadataMap = HashMap<String, String>;

/// Enhanced account kernel module with flexible authentication
pub struct EnhancedAccountModule {
    auth_manager: AuthManager,
}

impl EnhancedAccountModule {
    /// Create a new enhanced account module with standard authentication
    pub fn new_standard() -> Self {
        let mut auth_manager = AuthManager::new();
        
        // Add all signature authenticators
        for authenticator in create_default_signature_authenticators() {
            auth_manager.add_authenticator(authenticator);
        }
        
        // Add MFA authenticators
        for authenticator in create_default_mfa_authenticators() {
            auth_manager.add_authenticator(authenticator);
        }
        
        // Add standard policy
        auth_manager.add_policy(Box::new(StandardAccountPolicy));
        
        Self { auth_manager }
    }
    
    /// Create a new enhanced account module with high-security authentication
    pub fn new_high_security() -> Self {
        let mut auth_manager = AuthManager::new();
        
        // Add all signature authenticators
        for authenticator in create_default_signature_authenticators() {
            auth_manager.add_authenticator(authenticator);
        }
        
        // Add MFA authenticators
        for authenticator in create_default_mfa_authenticators() {
            auth_manager.add_authenticator(authenticator);
        }
        
        // Add high-security policy
        auth_manager.add_policy(Box::new(HighSecurityPolicy));
        
        Self { auth_manager }
    }
    
    /// Create a new enhanced account module with custom configuration
    pub fn new_custom(auth_manager: AuthManager) -> Self {
        Self { auth_manager }
    }
    
    /// Authenticate operation with provided credentials
    fn authenticate_operation(
        &self,
        operation: &str,
        target_account: UnitsObjectId,
        requester: UnitsObjectId,
        timestamp: u64,
        operation_data: &[u8],
        credentials: &[crate::auth::AuthCredential],
    ) -> Result<(), KernelError> {
        let auth_context = AuthContext {
            operation: operation.to_string(),
            target_account,
            requester,
            timestamp,
            operation_data: operation_data.to_vec(),
        };
        
        match self.auth_manager.authenticate(credentials, &auth_context) {
            AuthResult::Success => Ok(()),
            AuthResult::Failed(AuthError::InsufficientAuth) => Err(KernelError::Unauthorized),
            AuthResult::Failed(AuthError::InvalidCredentials) => Err(KernelError::Unauthorized),
            AuthResult::Failed(_) => Err(KernelError::InvalidParams),
            AuthResult::Pending(_) => {
                // For now, we don't support multi-step authentication in the kernel
                // This could be implemented by storing pending sessions in storage
                Err(KernelError::InvalidParams)
            }
        }
    }
}

impl KernelModule for EnhancedAccountModule {
    fn execute(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        // For now, create a standard auth manager since we can't store state
        // In a real implementation, you'd want to persist the auth manager configuration
        let module = Self::new_standard();
        
        match ctx.instruction.target_function.as_str() {
            "flex_create_account" => module.handle_flex_create_account(ctx),
            "flex_update_account" => module.handle_flex_update_account(ctx),
            "flex_add_recovery_address" => module.handle_flex_add_recovery_address(ctx),
            "flex_remove_recovery_address" => module.handle_flex_remove_recovery_address(ctx),
            "flex_deactivate_account" => module.handle_flex_deactivate_account(ctx),
            "flex_reactivate_account" => module.handle_flex_reactivate_account(ctx),
            "get_account" => module.handle_get_account(ctx),
            _ => Err(KernelError::InvalidFunction),
        }
    }
}

impl EnhancedAccountModule {
    fn handle_flex_create_account(&self, ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        let params: FlexCreateAccountParams = borsh::from_slice(&ctx.instruction.params)
            .map_err(|_| KernelError::InvalidData)?;
        
        if ctx.instruction.target_objects.is_empty() {
            return Err(KernelError::InvalidParams);
        }
        
        let account_id = ctx.instruction.target_objects[0];
        
        // Validate username if provided
        if let Some(ref username) = params.username {
            if !validate_username(&username) {
                return Err(KernelError::InvalidParams);
            }
        }
        
        // Authenticate the operation
        let operation_data = borsh::to_vec(&FlexCreateAccountParams {
            username: params.username.clone(),
            display_name: params.display_name.clone(),
            metadata: params.metadata.clone(),
            recovery_addresses: params.recovery_addresses.clone(),
            credentials: vec![], // Exclude credentials from message
        }).map_err(|_| KernelError::InvalidData)?;
        
        self.authenticate_operation(
            "create_account",
            account_id,
            ctx.instruction.controller_id,
            ctx.timestamp,
            &operation_data,
            &params.credentials,
        )?;
        
        // Create enhanced account data
        let mut account_data = EnhancedAccountData::new(account_id, ctx.timestamp);
        
        if let Some(username) = params.username {
            account_data.username = Some(username);
        }
        
        if let Some(display_name) = params.display_name {
            account_data.display_name = Some(display_name);
        }
        
        if let Some(metadata) = params.metadata {
            account_data.metadata = convert_metadata(metadata);
        }
        
        if let Some(recovery_addresses) = params.recovery_addresses {
            account_data.recovery_addresses = recovery_addresses;
        }
        
        let account_object = UnitsObject {
            id: account_id,
            controller_id: ctx.instruction.controller_id,
            object_type: ObjectType::Data,
            data: borsh::to_vec(&account_data)
                .map_err(|_| KernelError::InvalidData)?,
        };
        
        Ok(vec![ObjectEffect::creation(account_object)])
    }
    
    fn handle_flex_update_account(&self, ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        let params: FlexUpdateAccountParams = borsh::from_slice(&ctx.instruction.params)
            .map_err(|_| KernelError::InvalidData)?;
        
        let account = ctx.objects.get(&params.account_id)
            .ok_or(KernelError::ObjectNotFound)?;
        
        // Check authorization
        if account.controller_id != ctx.instruction.controller_id {
            return Err(KernelError::Unauthorized);
        }
        
        // Authenticate the operation
        let operation_data = borsh::to_vec(&FlexUpdateAccountParams {
            account_id: params.account_id,
            username: params.username.clone(),
            display_name: params.display_name.clone(),
            metadata: params.metadata.clone(),
            credentials: vec![], // Exclude credentials from message
        }).map_err(|_| KernelError::InvalidData)?;
        
        self.authenticate_operation(
            "update_account",
            params.account_id,
            ctx.instruction.controller_id,
            ctx.timestamp,
            &operation_data,
            &params.credentials,
        )?;
        
        let mut account_data: EnhancedAccountData = borsh::from_slice(&account.data)
            .map_err(|_| KernelError::InvalidData)?;
        
        // Check if account is active
        if !account_data.is_active {
            return Err(KernelError::InvalidParams);
        }
        
        // Validate username if provided
        if let Some(ref username) = params.username {
            if !validate_username(&username) {
                return Err(KernelError::InvalidParams);
            }
            account_data.username = Some(username.clone());
        }
        
        if let Some(display_name) = params.display_name {
            account_data.display_name = Some(display_name);
        }
        
        if let Some(metadata) = params.metadata {
            account_data.metadata = convert_metadata(metadata);
        }
        
        account_data.updated_at = ctx.timestamp;
        
        let updated_account = UnitsObject {
            id: account.id,
            controller_id: account.controller_id,
            object_type: account.object_type.clone(),
            data: borsh::to_vec(&account_data)
                .map_err(|_| KernelError::InvalidData)?,
        };
        
        Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
    }
    
    fn handle_flex_add_recovery_address(&self, ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        let params: FlexAddRecoveryAddressParams = borsh::from_slice(&ctx.instruction.params)
            .map_err(|_| KernelError::InvalidData)?;
        
        let account = ctx.objects.get(&params.account_id)
            .ok_or(KernelError::ObjectNotFound)?;
        
        // Check authorization
        if account.controller_id != ctx.instruction.controller_id {
            return Err(KernelError::Unauthorized);
        }
        
        // Authenticate the operation
        let operation_data = borsh::to_vec(&FlexAddRecoveryAddressParams {
            account_id: params.account_id,
            recovery_address: params.recovery_address,
            credentials: vec![], // Exclude credentials from message
        }).map_err(|_| KernelError::InvalidData)?;
        
        self.authenticate_operation(
            "add_recovery_address",
            params.account_id,
            ctx.instruction.controller_id,
            ctx.timestamp,
            &operation_data,
            &params.credentials,
        )?;
        
        let mut account_data: EnhancedAccountData = borsh::from_slice(&account.data)
            .map_err(|_| KernelError::InvalidData)?;
        
        // Check if account is active
        if !account_data.is_active {
            return Err(KernelError::InvalidParams);
        }
        
        // Check if recovery address already exists
        if account_data.recovery_addresses.contains(&params.recovery_address) {
            return Err(KernelError::InvalidParams);
        }
        
        account_data.recovery_addresses.push(params.recovery_address);
        account_data.updated_at = ctx.timestamp;
        
        let updated_account = UnitsObject {
            id: account.id,
            controller_id: account.controller_id,
            object_type: account.object_type.clone(),
            data: borsh::to_vec(&account_data)
                .map_err(|_| KernelError::InvalidData)?,
        };
        
        Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
    }
    
    fn handle_flex_remove_recovery_address(&self, ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        let params: FlexRemoveRecoveryAddressParams = borsh::from_slice(&ctx.instruction.params)
            .map_err(|_| KernelError::InvalidData)?;
        
        let account = ctx.objects.get(&params.account_id)
            .ok_or(KernelError::ObjectNotFound)?;
        
        // Check authorization
        if account.controller_id != ctx.instruction.controller_id {
            return Err(KernelError::Unauthorized);
        }
        
        // Authenticate the operation
        let operation_data = borsh::to_vec(&FlexRemoveRecoveryAddressParams {
            account_id: params.account_id,
            recovery_address: params.recovery_address,
            credentials: vec![], // Exclude credentials from message
        }).map_err(|_| KernelError::InvalidData)?;
        
        self.authenticate_operation(
            "remove_recovery_address",
            params.account_id,
            ctx.instruction.controller_id,
            ctx.timestamp,
            &operation_data,
            &params.credentials,
        )?;
        
        let mut account_data: EnhancedAccountData = borsh::from_slice(&account.data)
            .map_err(|_| KernelError::InvalidData)?;
        
        // Check if account is active
        if !account_data.is_active {
            return Err(KernelError::InvalidParams);
        }
        
        // Find and remove recovery address
        let initial_len = account_data.recovery_addresses.len();
        account_data.recovery_addresses.retain(|&addr| addr != params.recovery_address);
        
        if account_data.recovery_addresses.len() == initial_len {
            return Err(KernelError::InvalidParams);
        }
        
        account_data.updated_at = ctx.timestamp;
        
        let updated_account = UnitsObject {
            id: account.id,
            controller_id: account.controller_id,
            object_type: account.object_type.clone(),
            data: borsh::to_vec(&account_data)
                .map_err(|_| KernelError::InvalidData)?,
        };
        
        Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
    }
    
    fn handle_flex_deactivate_account(&self, ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        let params: FlexDeactivateAccountParams = borsh::from_slice(&ctx.instruction.params)
            .map_err(|_| KernelError::InvalidData)?;
        
        let account = ctx.objects.get(&params.account_id)
            .ok_or(KernelError::ObjectNotFound)?;
        
        // Check authorization
        if account.controller_id != ctx.instruction.controller_id {
            return Err(KernelError::Unauthorized);
        }
        
        // Authenticate the operation
        let operation_data = borsh::to_vec(&FlexDeactivateAccountParams {
            account_id: params.account_id,
            credentials: vec![], // Exclude credentials from message
        }).map_err(|_| KernelError::InvalidData)?;
        
        self.authenticate_operation(
            "deactivate_account",
            params.account_id,
            ctx.instruction.controller_id,
            ctx.timestamp,
            &operation_data,
            &params.credentials,
        )?;
        
        let mut account_data: EnhancedAccountData = borsh::from_slice(&account.data)
            .map_err(|_| KernelError::InvalidData)?;
        
        // Check if already inactive
        if !account_data.is_active {
            return Err(KernelError::InvalidParams);
        }
        
        account_data.is_active = false;
        account_data.updated_at = ctx.timestamp;
        
        let updated_account = UnitsObject {
            id: account.id,
            controller_id: account.controller_id,
            object_type: account.object_type.clone(),
            data: borsh::to_vec(&account_data)
                .map_err(|_| KernelError::InvalidData)?,
        };
        
        Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
    }
    
    fn handle_flex_reactivate_account(&self, ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        let params: FlexReactivateAccountParams = borsh::from_slice(&ctx.instruction.params)
            .map_err(|_| KernelError::InvalidData)?;
        
        let account = ctx.objects.get(&params.account_id)
            .ok_or(KernelError::ObjectNotFound)?;
        
        let account_data: EnhancedAccountData = borsh::from_slice(&account.data)
            .map_err(|_| KernelError::InvalidData)?;
        
        // Check authorization (controller or recovery address)
        let is_controller = account.controller_id == ctx.instruction.controller_id;
        let is_recovery = account_data.recovery_addresses.contains(&ctx.instruction.controller_id);
        
        if !is_controller && !is_recovery {
            return Err(KernelError::Unauthorized);
        }
        
        // Authenticate the operation
        let operation_data = borsh::to_vec(&FlexReactivateAccountParams {
            account_id: params.account_id,
            credentials: vec![], // Exclude credentials from message
        }).map_err(|_| KernelError::InvalidData)?;
        
        self.authenticate_operation(
            "reactivate_account",
            params.account_id,
            ctx.instruction.controller_id,
            ctx.timestamp,
            &operation_data,
            &params.credentials,
        )?;
        
        // Check if already active
        if account_data.is_active {
            return Err(KernelError::InvalidParams);
        }
        
        let mut updated_data = account_data;
        updated_data.is_active = true;
        updated_data.updated_at = ctx.timestamp;
        
        let updated_account = UnitsObject {
            id: account.id,
            controller_id: account.controller_id,
            object_type: account.object_type.clone(),
            data: borsh::to_vec(&updated_data)
                .map_err(|_| KernelError::InvalidData)?,
        };
        
        Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
    }
    
    fn handle_get_account(&self, ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        let params: GetAccountParams = borsh::from_slice(&ctx.instruction.params)
            .map_err(|_| KernelError::InvalidData)?;
        
        // This is a read-only operation, just verify the account exists
        let _account = ctx.objects.get(&params.account_id)
            .ok_or(KernelError::ObjectNotFound)?;
        
        // No effects for read-only operation
        Ok(vec![])
    }
}

#[cfg(not(feature = "std"))]
fn convert_metadata(metadata: HashMap<String, String>) -> BTreeMap<String, String> {
    metadata.into_iter().collect()
}

#[cfg(feature = "std")]
fn convert_metadata(metadata: HashMap<String, String>) -> HashMap<String, String> {
    metadata
}