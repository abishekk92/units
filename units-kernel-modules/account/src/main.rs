#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec, string::ToString};

#[cfg(feature = "std")]
use std::{vec, vec::Vec};

#[cfg(not(feature = "std"))]
units_kernel_sdk::use_default_allocator!();

use account::{
    AccountData, CreateAccountParams, UpdateAccountParams, AddRecoveryAddressParams,
    RemoveRecoveryAddressParams, DeactivateAccountParams, ReactivateAccountParams,
    validate_username,
};
use units_kernel_sdk::{
    ExecutionContext, ObjectEffect, KernelModule, KernelError,
    UnitsObject, ObjectType,
};
#[cfg(not(feature = "std"))]
use units_kernel_sdk::{read_context, write_effects};

/// Account kernel module implementation
struct AccountModule;

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
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let account_id = ctx.instruction.target_objects[0];
    
    // Check if account already exists
    if ctx.objects.contains_key(&account_id) {
        return Err(KernelError::InvalidParams); // Account already exists
    }
    
    // Validate username if provided
    if let Some(ref username) = params.username {
        if !validate_username(username) {
            return Err(KernelError::InvalidParams);
        }
        
        // In a real implementation, we would check username uniqueness
        // This would require a username registry object
    }
    
    let account_data = AccountData {
        account_id,
        username: params.username,
        display_name: params.display_name,
        metadata: params.metadata,
        is_active: true,
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
        recovery_addresses: params.recovery_addresses,
    };
    
    let account_object = UnitsObject {
        id: account_id,
        controller_id: ctx.instruction.controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&account_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::creation(account_object)])
}

fn handle_update_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: UpdateAccountParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let account_id = ctx.instruction.target_objects[0];
    let account = ctx.objects.get(&account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Verify controller
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    // Check if account is active
    if !account_data.is_active {
        return Err(KernelError::InvalidParams); // Account inactive
    }
    
    // Update fields if provided
    if let Some(username) = params.username {
        if !validate_username(&username) {
            return Err(KernelError::InvalidParams);
        }
        account_data.username = Some(username);
    }
    
    if let Some(display_name) = params.display_name {
        account_data.display_name = display_name;
    }
    
    if let Some(metadata) = params.metadata {
        account_data.metadata = metadata;
    }
    
    account_data.updated_at = ctx.timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn handle_add_recovery_address(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: AddRecoveryAddressParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let account_id = ctx.instruction.target_objects[0];
    let account = ctx.objects.get(&account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Verify controller
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    // Check if recovery address already exists
    if account_data.recovery_addresses.contains(&params.recovery_address) {
        return Err(KernelError::InvalidParams); // Recovery address already exists
    }
    
    account_data.recovery_addresses.push(params.recovery_address);
    account_data.updated_at = ctx.timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn handle_remove_recovery_address(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: RemoveRecoveryAddressParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let account_id = ctx.instruction.target_objects[0];
    let account = ctx.objects.get(&account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Verify controller
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    // Find and remove the recovery address
    let initial_len = account_data.recovery_addresses.len();
    account_data.recovery_addresses.retain(|addr| addr != &params.recovery_address);
    
    if account_data.recovery_addresses.len() == initial_len {
        return Err(KernelError::InvalidParams); // Recovery address not found
    }
    
    account_data.updated_at = ctx.timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn handle_deactivate_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let _params: DeactivateAccountParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let account_id = ctx.instruction.target_objects[0];
    let account = ctx.objects.get(&account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Verify controller
    if account.controller_id != ctx.instruction.controller_id {
        return Err(KernelError::Unauthorized);
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    if !account_data.is_active {
        return Err(KernelError::InvalidParams); // Already inactive
    }
    
    account_data.is_active = false;
    account_data.updated_at = ctx.timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn handle_reactivate_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let _params: ReactivateAccountParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let account_id = ctx.instruction.target_objects[0];
    let account = ctx.objects.get(&account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Verify controller or recovery address
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    let is_authorized = account.controller_id == ctx.instruction.controller_id ||
        account_data.recovery_addresses.contains(&ctx.instruction.controller_id);
    
    if !is_authorized {
        return Err(KernelError::Unauthorized);
    }
    
    if account_data.is_active {
        return Err(KernelError::InvalidParams); // Already active
    }
    
    account_data.is_active = true;
    account_data.updated_at = ctx.timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn handle_get_account(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    // This is a read-only operation, return empty effects
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let account_id = ctx.instruction.target_objects[0];
    let _account = ctx.objects.get(&account_id)
        .ok_or(KernelError::ObjectNotFound)?;
    
    // In a real implementation, this might emit an event or log
    // For now, just return empty effects since it's read-only
    Ok(vec![])
}

/// Entry point for the kernel module  
#[cfg(not(feature = "std"))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Read execution context from standard input
    let ctx = match read_context() {
        Ok(ctx) => ctx,
        Err(_) => units_kernel_sdk::exit(KernelError::InvalidParams as i32),
    };
    
    // Execute the module
    let effects = match AccountModule::execute(&ctx) {
        Ok(effects) => effects,
        Err(e) => units_kernel_sdk::exit(e as i32),
    };
    
    // Write effects to standard output
    match write_effects(&effects) {
        Ok(_) => units_kernel_sdk::exit(0),
        Err(_) => units_kernel_sdk::exit(KernelError::IOError as i32),
    }
}

/// Entry point for std builds (testing)
#[cfg(feature = "std")]
fn main() {
    println!("Account kernel module - std build for testing");
}

/// Panic handler for no_std environment
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    units_kernel_sdk::exit(KernelError::Panic as i32)
}