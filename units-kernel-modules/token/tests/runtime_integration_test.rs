use std::collections::HashMap;
use token::*;
use units_core::{
    constants::TOKEN_CONTROLLER_ID,
    Instruction, UnitsObject, UnitsObjectId, ObjectType,
};
use units_runtime::{
    ExecutionContext, ObjectEffect, MockRuntime, VMExecutor, VMExecutionError,
};

/// Test that simulates the actual runtime execution flow
#[tokio::test]
async fn test_token_lifecycle_with_runtime() {
    // Create mock runtime
    let mut runtime = MockRuntime::new();
    
    // Generate test IDs
    let token_id = UnitsObjectId::unique_id_for_tests();
    let alice_balance_id = UnitsObjectId::unique_id_for_tests();
    let bob_balance_id = UnitsObjectId::unique_id_for_tests();
    let alice_owner_id = UnitsObjectId::unique_id_for_tests();
    let bob_owner_id = UnitsObjectId::unique_id_for_tests();
    
    // Test 1: Tokenize - Create new token
    println!("=== Test 1: Tokenize ===");
    
    let tokenize_params = TokenizeParams {
        initial_supply: 1_000_000,
        decimals: 18,
        name: "Test Token".to_string(),
        symbol: "TEST".to_string(),
    };
    
    let tokenize_instruction = Instruction::new(
        TOKEN_CONTROLLER_ID,
        "tokenize".to_string(),
        vec![token_id, alice_balance_id],
        borsh::to_vec(&tokenize_params).unwrap(),
    );
    
    // Simulate runtime execution context
    let mut objects = HashMap::new();
    let context = ExecutionContext::new(
        tokenize_instruction,
        objects.clone(),
        1, // slot
        1234567890, // timestamp
    );
    
    // Simulate kernel module execution and verify results
    let effects = simulate_tokenize_execution(&context).unwrap();
    assert_eq!(effects.len(), 2);
    
    // Apply effects to runtime state
    for effect in &effects {
        if let Some(after_image) = &effect.after_image {
            objects.insert(effect.object_id, after_image.clone());
        }
    }
    
    // Verify token was created correctly
    let token_obj = objects.get(&token_id).unwrap();
    let token_data: TokenData = borsh::from_slice(&token_obj.data).unwrap();
    assert_eq!(token_data.total_supply, 1_000_000);
    assert_eq!(token_data.symbol, "TEST");
    assert!(!token_data.is_frozen);
    
    // Verify Alice's initial balance
    let alice_balance_obj = objects.get(&alice_balance_id).unwrap();
    let alice_balance: BalanceData = borsh::from_slice(&alice_balance_obj.data).unwrap();
    assert_eq!(alice_balance.amount, 1_000_000);
    assert_eq!(alice_balance.token_id, token_id);
    
    println!("✓ Token created successfully with {} {} tokens", token_data.total_supply, token_data.symbol);
    
    // Test 2: Transfer - Create Bob's balance and transfer tokens
    println!("\n=== Test 2: Transfer ===");
    
    // Create Bob's balance object (initially empty)
    let bob_balance_data = BalanceData {
        token_id,
        owner_id: bob_owner_id,
        amount: 0,
    };
    
    let bob_balance_obj = UnitsObject {
        id: bob_balance_id,
        controller_id: TOKEN_CONTROLLER_ID,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&bob_balance_data).unwrap(),
    };
    
    objects.insert(bob_balance_id, bob_balance_obj);
    
    // Transfer 100,000 tokens from Alice to Bob
    let transfer_params = TransferParams { amount: 100_000 };
    
    let transfer_instruction = Instruction::new(
        TOKEN_CONTROLLER_ID,
        "transfer".to_string(),
        vec![token_id, alice_balance_id, bob_balance_id],
        borsh::to_vec(&transfer_params).unwrap(),
    );
    
    let context = ExecutionContext::new(
        transfer_instruction,
        objects.clone(),
        2, // slot
        1234567900, // timestamp
    );
    
    let effects = simulate_transfer_execution(&context).unwrap();
    assert_eq!(effects.len(), 2);
    
    // Apply transfer effects
    for effect in &effects {
        if let Some(after_image) = &effect.after_image {
            objects.insert(effect.object_id, after_image.clone());
        }
    }
    
    // Verify balances after transfer
    let alice_balance_obj = objects.get(&alice_balance_id).unwrap();
    let alice_balance: BalanceData = borsh::from_slice(&alice_balance_obj.data).unwrap();
    assert_eq!(alice_balance.amount, 900_000);
    
    let bob_balance_obj = objects.get(&bob_balance_id).unwrap();
    let bob_balance: BalanceData = borsh::from_slice(&bob_balance_obj.data).unwrap();
    assert_eq!(bob_balance.amount, 100_000);
    
    println!("✓ Transferred {} tokens: Alice={}, Bob={}", 
             transfer_params.amount, alice_balance.amount, bob_balance.amount);
    
    // Test 3: Mint - Increase token supply
    println!("\n=== Test 3: Mint ===");
    
    let mint_params = MintParams { amount: 500_000 };
    
    let mint_instruction = Instruction::new(
        TOKEN_CONTROLLER_ID,
        "mint".to_string(),
        vec![token_id, alice_balance_id],
        borsh::to_vec(&mint_params).unwrap(),
    );
    
    let context = ExecutionContext::new(
        mint_instruction,
        objects.clone(),
        3, // slot
        1234567910, // timestamp
    );
    
    let effects = simulate_mint_execution(&context).unwrap();
    assert_eq!(effects.len(), 2);
    
    // Apply mint effects
    for effect in &effects {
        if let Some(after_image) = &effect.after_image {
            objects.insert(effect.object_id, after_image.clone());
        }
    }
    
    // Verify total supply and Alice's balance increased
    let token_obj = objects.get(&token_id).unwrap();
    let token_data: TokenData = borsh::from_slice(&token_obj.data).unwrap();
    assert_eq!(token_data.total_supply, 1_500_000);
    
    let alice_balance_obj = objects.get(&alice_balance_id).unwrap();
    let alice_balance: BalanceData = borsh::from_slice(&alice_balance_obj.data).unwrap();
    assert_eq!(alice_balance.amount, 1_400_000);
    
    println!("✓ Minted {} tokens: Total supply={}, Alice balance={}",
             mint_params.amount, token_data.total_supply, alice_balance.amount);
    
    // Test 4: Freeze/Unfreeze
    println!("\n=== Test 4: Freeze/Unfreeze ===");
    
    let freeze_instruction = Instruction::new(
        TOKEN_CONTROLLER_ID,
        "freeze".to_string(),
        vec![token_id],
        vec![], // No parameters
    );
    
    let context = ExecutionContext::new(
        freeze_instruction,
        objects.clone(),
        4, // slot
        1234567920, // timestamp
    );
    
    let effects = simulate_freeze_execution(&context).unwrap();
    assert_eq!(effects.len(), 1);
    
    // Apply freeze effect
    for effect in &effects {
        if let Some(after_image) = &effect.after_image {
            objects.insert(effect.object_id, after_image.clone());
        }
    }
    
    // Verify token is frozen
    let token_obj = objects.get(&token_id).unwrap();
    let token_data: TokenData = borsh::from_slice(&token_obj.data).unwrap();
    assert!(token_data.is_frozen);
    
    println!("✓ Token frozen successfully");
    
    // Test transfer while frozen (should fail)
    let failed_transfer = Instruction::new(
        TOKEN_CONTROLLER_ID,
        "transfer".to_string(),
        vec![token_id, alice_balance_id, bob_balance_id],
        borsh::to_vec(&TransferParams { amount: 1000 }).unwrap(),
    );
    
    let context = ExecutionContext::new(
        failed_transfer,
        objects.clone(),
        5, // slot
        1234567930, // timestamp
    );
    
    let result = simulate_transfer_execution(&context);
    assert!(result.is_err());
    println!("✓ Transfer correctly blocked while frozen");
    
    println!("\n=== All Runtime Integration Tests Passed! ===");
}

// Helper functions to simulate kernel module execution
// In a real system, these would be handled by the RISC-V VM

fn simulate_tokenize_execution(context: &ExecutionContext) -> Result<Vec<ObjectEffect>, VMExecutionError> {
    let params: TokenizeParams = borsh::from_slice(&context.instruction.params)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid tokenize parameters".to_string()))?;
    
    if context.instruction.target_objects.len() < 2 {
        return Err(VMExecutionError::ExecutionFailed("Insufficient target objects".to_string()));
    }
    
    let token_data = TokenData {
        total_supply: params.initial_supply,
        decimals: params.decimals,
        name: params.name,
        symbol: params.symbol,
        is_frozen: false,
    };
    
    let balance_data = BalanceData {
        token_id: context.instruction.target_objects[0],
        owner_id: context.instruction.target_objects[1],
        amount: params.initial_supply,
    };
    
    let token_object = UnitsObject {
        id: context.instruction.target_objects[0],
        controller_id: TOKEN_CONTROLLER_ID,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&token_data).unwrap(),
    };
    
    let balance_object = UnitsObject {
        id: context.instruction.target_objects[1],
        controller_id: TOKEN_CONTROLLER_ID,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&balance_data).unwrap(),
    };
    
    Ok(vec![
        ObjectEffect::creation(token_object),
        ObjectEffect::creation(balance_object),
    ])
}

fn simulate_transfer_execution(context: &ExecutionContext) -> Result<Vec<ObjectEffect>, VMExecutionError> {
    let params: TransferParams = borsh::from_slice(&context.instruction.params)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid transfer parameters".to_string()))?;
    
    if context.instruction.target_objects.len() < 3 {
        return Err(VMExecutionError::ExecutionFailed("Insufficient target objects".to_string()));
    }
    
    let token = context.objects.get(&context.instruction.target_objects[0])
        .ok_or_else(|| VMExecutionError::ExecutionFailed("Token not found".to_string()))?;
    let from_balance = context.objects.get(&context.instruction.target_objects[1])
        .ok_or_else(|| VMExecutionError::ExecutionFailed("From balance not found".to_string()))?;
    let to_balance = context.objects.get(&context.instruction.target_objects[2])
        .ok_or_else(|| VMExecutionError::ExecutionFailed("To balance not found".to_string()))?;
    
    let token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid token data".to_string()))?;
    let mut from_data: BalanceData = borsh::from_slice(&from_balance.data)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid from balance data".to_string()))?;
    let mut to_data: BalanceData = borsh::from_slice(&to_balance.data)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid to balance data".to_string()))?;
    
    if token_data.is_frozen {
        return Err(VMExecutionError::ExecutionFailed("Token is frozen".to_string()));
    }
    
    if from_data.amount < params.amount {
        return Err(VMExecutionError::ExecutionFailed("Insufficient balance".to_string()));
    }
    
    from_data.amount -= params.amount;
    to_data.amount += params.amount;
    
    let updated_from = UnitsObject {
        id: from_balance.id,
        controller_id: from_balance.controller_id,
        object_type: from_balance.object_type.clone(),
        data: borsh::to_vec(&from_data).unwrap(),
    };
    
    let updated_to = UnitsObject {
        id: to_balance.id,
        controller_id: to_balance.controller_id,
        object_type: to_balance.object_type.clone(),
        data: borsh::to_vec(&to_data).unwrap(),
    };
    
    Ok(vec![
        ObjectEffect::modification(from_balance.clone(), updated_from),
        ObjectEffect::modification(to_balance.clone(), updated_to),
    ])
}

fn simulate_mint_execution(context: &ExecutionContext) -> Result<Vec<ObjectEffect>, VMExecutionError> {
    let params: MintParams = borsh::from_slice(&context.instruction.params)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid mint parameters".to_string()))?;
    
    if context.instruction.target_objects.len() < 2 {
        return Err(VMExecutionError::ExecutionFailed("Insufficient target objects".to_string()));
    }
    
    let token = context.objects.get(&context.instruction.target_objects[0])
        .ok_or_else(|| VMExecutionError::ExecutionFailed("Token not found".to_string()))?;
    let balance = context.objects.get(&context.instruction.target_objects[1])
        .ok_or_else(|| VMExecutionError::ExecutionFailed("Balance not found".to_string()))?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid token data".to_string()))?;
    let mut balance_data: BalanceData = borsh::from_slice(&balance.data)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid balance data".to_string()))?;
    
    token_data.total_supply += params.amount;
    balance_data.amount += params.amount;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).unwrap(),
    };
    
    let updated_balance = UnitsObject {
        id: balance.id,
        controller_id: balance.controller_id,
        object_type: balance.object_type.clone(),
        data: borsh::to_vec(&balance_data).unwrap(),
    };
    
    Ok(vec![
        ObjectEffect::modification(token.clone(), updated_token),
        ObjectEffect::modification(balance.clone(), updated_balance),
    ])
}

fn simulate_freeze_execution(context: &ExecutionContext) -> Result<Vec<ObjectEffect>, VMExecutionError> {
    if context.instruction.target_objects.is_empty() {
        return Err(VMExecutionError::ExecutionFailed("No target objects".to_string()));
    }
    
    let token = context.objects.get(&context.instruction.target_objects[0])
        .ok_or_else(|| VMExecutionError::ExecutionFailed("Token not found".to_string()))?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| VMExecutionError::ExecutionFailed("Invalid token data".to_string()))?;
    
    token_data.is_frozen = true;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(token.clone(), updated_token)])
}