#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec, string::String, collections::BTreeMap};

#[cfg(feature = "std")]
use std::{vec, vec::Vec, string::String, collections::HashMap};

#[cfg(not(feature = "std"))]
units_kernel_sdk::use_default_allocator!();

use crate::{
    AccountData, CreateAccountParams, UpdateAccountParams, AddRecoveryAddressParams,
    RemoveRecoveryAddressParams, DeactivateAccountParams, ReactivateAccountParams,
    GetAccountParams, validate_username, 
    FN_CREATE_ACCOUNT, FN_UPDATE_ACCOUNT, FN_ADD_RECOVERY_ADDRESS, FN_REMOVE_RECOVERY_ADDRESS,
    FN_DEACTIVATE_ACCOUNT, FN_REACTIVATE_ACCOUNT, FN_GET_ACCOUNT,
    crypto::{verify_signature, create_operation_message, PublicKey, CryptoError},
};
use units_kernel_sdk::{
    ExecutionContext, ObjectEffect, KernelModule, KernelError,
    read_context, write_effects, UnitsObject, ObjectType, UnitsObjectId,
};

#[cfg(not(feature = "std"))]
type MetadataMap = BTreeMap<String, String>;

#[cfg(feature = "std")]
type MetadataMap = HashMap<String, String>;

/// Account kernel module implementation
pub struct AccountModule;

impl KernelModule for AccountModule {
    fn execute(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        match ctx.instruction.target_function.as_str() {
            "create_account" => handle_create_account(ctx),
            "update_account" => handle_update_account(ctx),
            "add_recovery_address" => handle_add_recovery_address(ctx),
            "remove_recovery_address" => handle_remove_recovery_address(ctx),
            "deactivate_account" => handle_deactivate_account(ctx),
            "reactivate_account" => handle_reactivate_account(ctx),
            "get_account" => handle_get_account(ctx),
            _ => Err(KernelError::InvalidFunction),
        }
    }
}

fn handle_create_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: CreateAccountParams = borsh::from_slice(&ctx.instruction.params)
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
    
    // Create account data
    let mut account_data = AccountData::new(account_id, ctx.timestamp);
    
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

fn handle_update_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: UpdateAccountParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidData)?;
    
    let account = ctx.objects.get(&params.account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Check authorization
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    // Verify signature - the signature should be from the account owner (controller)
    let operation_params = borsh::to_vec(&UpdateAccountParams {
        account_id: params.account_id,
        username: params.username.clone(),
        display_name: params.display_name.clone(),
        metadata: params.metadata.clone(),
        signature: crate::crypto::Signature::new([0u8; 64]), // Exclude signature from message
    }).map_err(|_| KernelError::InvalidData)?;
    
    verify_account_signature(
        &ctx.instruction.controller_id,
        "update_account",
        &params.account_id,
        ctx.timestamp,
        &operation_params,
        &params.signature,
    )?;
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
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

fn handle_add_recovery_address(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: AddRecoveryAddressParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidData)?;
    
    let account = ctx.objects.get(&params.account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Check authorization
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
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

fn handle_remove_recovery_address(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: RemoveRecoveryAddressParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidData)?;
    
    let account = ctx.objects.get(&params.account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Check authorization
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
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

fn handle_deactivate_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: DeactivateAccountParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidData)?;
    
    let account = ctx.objects.get(&params.account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Check authorization
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
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

fn handle_reactivate_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: ReactivateAccountParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidData)?;
    
    let account = ctx.objects.get(&params.account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    let account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    // Check authorization (controller or recovery address)
    let is_controller = account.controller_id == ctx.instruction.controller_id;
    let is_recovery = account_data.recovery_addresses.contains(&ctx.instruction.controller_id);
    
    if !is_controller && !is_recovery {
        return Err(KernelError::Unauthorized);
    }
    
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

fn handle_get_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: GetAccountParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidData)?;
    
    // This is a read-only operation, just verify the account exists
    let _account = ctx.objects.get(&params.account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // No effects for read-only operation
    Ok(vec![])
}

#[cfg(not(feature = "std"))]
fn convert_metadata(metadata: HashMap<String, String>) -> BTreeMap<String, String> {
    metadata.into_iter().collect()
}

#[cfg(feature = "std")]
fn convert_metadata(metadata: HashMap<String, String>) -> HashMap<String, String> {
    metadata
}

/// Verify signature for account operations
fn verify_account_signature(
    signer_id: &UnitsObjectId,
    operation: &str,
    account_id: &UnitsObjectId,
    timestamp: u64,
    params: &[u8],
    signature: &crate::crypto::Signature,
) -> Result<(), KernelError> {
    // Convert signer ID to public key
    let public_key = PublicKey::from_units_object_id(signer_id)
        .map_err(|_| KernelError::InvalidParams)?;
    
    // Create the message to verify
    let message = create_operation_message(operation, account_id, timestamp, params);
    
    // Verify the signature
    verify_signature(&public_key, &message, signature)
        .map_err(|err| match err {
            CryptoError::SignatureVerificationFailed => KernelError::Unauthorized,
            _ => KernelError::InvalidParams,
        })
}

