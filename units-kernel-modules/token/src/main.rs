#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

#[cfg(feature = "std")]
use std::{vec, vec::Vec};

#[cfg(not(feature = "std"))]
units_kernel_sdk::use_default_allocator!();

use token::{
    TokenData, BalanceData, TokenizeParams, TransferParams, MintParams, BurnParams,
};
use units_kernel_sdk::{
    ExecutionContext, ObjectEffect, KernelModule, KernelError,
    read_context, write_effects, UnitsObject, ObjectType,
};

/// Token kernel module implementation
struct TokenModule;

impl KernelModule for TokenModule {
    fn execute(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
        match ctx.instruction.target_function.as_str() {
            "create_token" => handle_create_token(ctx),
            "transfer_token" => handle_transfer_token(ctx),
            "mint_token" => handle_mint_token(ctx),
            "burn_token" => handle_burn_token(ctx),
            "freeze_token" => handle_freeze_token(ctx),
            "unfreeze_token" => handle_unfreeze_token(ctx),
            _ => Err(KernelError::InvalidFunction),
        }
    }
}

fn handle_create_token(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: TokenizeParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.len() < 2 {
        return Err(KernelError::InvalidParams);
    }
    
    let token_data = TokenData {
        total_supply: params.initial_supply,
        decimals: params.decimals,
        name: params.name,
        symbol: params.symbol,
        is_frozen: false,
    };
    
    let balance_data = BalanceData {
        token_id: ctx.instruction.target_objects[0],
        owner_id: ctx.instruction.target_objects[1],
        amount: params.initial_supply,
    };
    
    let token_object = UnitsObject {
        id: ctx.instruction.target_objects[0],
        controller_id: ctx.instruction.controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&token_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    let balance_object = UnitsObject {
        id: ctx.instruction.target_objects[1],
        controller_id: ctx.instruction.controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&balance_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![
        ObjectEffect::creation(token_object),
        ObjectEffect::creation(balance_object),
    ])
}

fn handle_transfer_token(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: TransferParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.len() < 3 {
        return Err(KernelError::InvalidParams);
    }
    
    // Get objects
    let token = ctx.objects.get(&ctx.instruction.target_objects[0])
        .ok_or(KernelError::ObjectNotFound)?;
    let from_balance = ctx.objects.get(&ctx.instruction.target_objects[1])
        .ok_or(KernelError::ObjectNotFound)?;
    let to_balance = ctx.objects.get(&ctx.instruction.target_objects[2])
        .ok_or(KernelError::ObjectNotFound)?;
    
    // Parse data
    let token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| KernelError::InvalidData)?;
    let mut from_data: BalanceData = borsh::from_slice(&from_balance.data)
        .map_err(|_| KernelError::InvalidData)?;
    let mut to_data: BalanceData = borsh::from_slice(&to_balance.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    // Validate
    if token_data.is_frozen {
        return Err(KernelError::TokenFrozen);
    }
    
    if from_data.amount < params.amount {
        return Err(KernelError::InsufficientBalance);
    }
    
    // Check for overflow
    to_data.amount = to_data.amount.checked_add(params.amount)
        .ok_or(KernelError::Overflow)?;
    from_data.amount -= params.amount;
    
    // Create effects
    let updated_from = UnitsObject {
        id: from_balance.id,
        controller_id: from_balance.controller_id,
        object_type: from_balance.object_type.clone(),
        data: borsh::to_vec(&from_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    let updated_to = UnitsObject {
        id: to_balance.id,
        controller_id: to_balance.controller_id,
        object_type: to_balance.object_type.clone(),
        data: borsh::to_vec(&to_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![
        ObjectEffect::modification(from_balance.clone(), updated_from),
        ObjectEffect::modification(to_balance.clone(), updated_to),
    ])
}

fn handle_mint_token(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: MintParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.len() < 2 {
        return Err(KernelError::InvalidParams);
    }
    
    let token = ctx.objects.get(&ctx.instruction.target_objects[0])
        .ok_or(KernelError::ObjectNotFound)?;
    let balance = ctx.objects.get(&ctx.instruction.target_objects[1])
        .ok_or(KernelError::ObjectNotFound)?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| KernelError::InvalidData)?;
    let mut balance_data: BalanceData = borsh::from_slice(&balance.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    // Check for overflow
    token_data.total_supply = token_data.total_supply.checked_add(params.amount)
        .ok_or(KernelError::Overflow)?;
    balance_data.amount = balance_data.amount.checked_add(params.amount)
        .ok_or(KernelError::Overflow)?;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    let updated_balance = UnitsObject {
        id: balance.id,
        controller_id: balance.controller_id,
        object_type: balance.object_type.clone(),
        data: borsh::to_vec(&balance_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![
        ObjectEffect::modification(token.clone(), updated_token),
        ObjectEffect::modification(balance.clone(), updated_balance),
    ])
}

fn handle_burn_token(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    let params: BurnParams = borsh::from_slice(&ctx.instruction.params)
        .map_err(|_| KernelError::InvalidParams)?;
    
    if ctx.instruction.target_objects.len() < 2 {
        return Err(KernelError::InvalidParams);
    }
    
    let token = ctx.objects.get(&ctx.instruction.target_objects[0])
        .ok_or(KernelError::ObjectNotFound)?;
    let balance = ctx.objects.get(&ctx.instruction.target_objects[1])
        .ok_or(KernelError::ObjectNotFound)?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| KernelError::InvalidData)?;
    let mut balance_data: BalanceData = borsh::from_slice(&balance.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    // Validate
    if balance_data.amount < params.amount {
        return Err(KernelError::InsufficientBalance);
    }
    if token_data.total_supply < params.amount {
        return Err(KernelError::InvalidParams);
    }
    
    token_data.total_supply -= params.amount;
    balance_data.amount -= params.amount;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    let updated_balance = UnitsObject {
        id: balance.id,
        controller_id: balance.controller_id,
        object_type: balance.object_type.clone(),
        data: borsh::to_vec(&balance_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![
        ObjectEffect::modification(token.clone(), updated_token),
        ObjectEffect::modification(balance.clone(), updated_balance),
    ])
}

fn handle_freeze_token(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let token = ctx.objects.get(&ctx.instruction.target_objects[0])
        .ok_or(KernelError::ObjectNotFound)?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    token_data.is_frozen = true;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::modification(token.clone(), updated_token)])
}

fn handle_unfreeze_token(ctx: &ExecutionContext) -> Result<Vec<ObjectEffect>, KernelError> {
    if ctx.instruction.target_objects.is_empty() {
        return Err(KernelError::InvalidParams);
    }
    
    let token = ctx.objects.get(&ctx.instruction.target_objects[0])
        .ok_or(KernelError::ObjectNotFound)?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| KernelError::InvalidData)?;
    
    token_data.is_frozen = false;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).map_err(|_| KernelError::InvalidData)?,
    };
    
    Ok(vec![ObjectEffect::modification(token.clone(), updated_token)])
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
    let effects = match TokenModule::execute(&ctx) {
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
    println!("Token kernel module - std build for testing");
}

/// Panic handler for no_std environment
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    units_kernel_sdk::exit(KernelError::Panic as i32)
}