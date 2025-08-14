use token::*;
use units_kernel_sdk::{Instruction, UnitsObject, UnitsObjectId, ObjectType, ObjectEffect, OBJECT_ID_SIZE};

/// Helper to create a test execution context
struct TestContext {
    objects: Vec<UnitsObject>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    fn add_object(&mut self, object: UnitsObject) {
        self.objects.push(object);
    }

    fn get_object(&self, id: &UnitsObjectId) -> Option<&UnitsObject> {
        self.objects.iter().find(|o| o.id == *id)
    }

    fn apply_effects(&mut self, effects: Vec<ObjectEffect>) {
        for effect in effects {
            // Handle object deletion
            if effect.after_image.is_none() {
                self.objects.retain(|o| o.id != effect.object_id);
                continue;
            }
            
            // Handle object creation or modification
            if let Some(after) = effect.after_image {
                if let Some(obj) = self.objects.iter_mut().find(|o| o.id == effect.object_id) {
                    // Update existing object
                    *obj = after;
                } else {
                    // Add new object
                    self.objects.push(after);
                }
            }
        }
    }
}

/// Simulates kernel module execution (in real system this would run in RISC-V VM)
fn simulate_kernel_execution(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, TokenError> {
    // In a real system, this would serialize the context, run the RISC-V module,
    // and deserialize the results. For testing, we simulate the logic.
    
    match instruction.target_function.as_str() {
        "create_token" => simulate_tokenize(instruction, context),
        "transfer_token" => simulate_transfer(instruction, context),
        "mint_token" => simulate_mint(instruction, context),
        "burn_token" => simulate_burn(instruction, context),
        "freeze_token" => simulate_freeze(instruction, context),
        "unfreeze_token" => simulate_unfreeze(instruction, context),
        _ => Err(TokenError::from_code(TokenError::INVALID_FUNCTION)),
    }
}

fn simulate_tokenize(
    instruction: &Instruction,
    _context: &TestContext,
) -> Result<Vec<ObjectEffect>, TokenError> {
    let params: TokenizeParams = borsh::from_slice(&instruction.params)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    if instruction.target_objects.len() < 2 {
        return Err(TokenError::from_code(TokenError::INVALID_PARAMS));
    }
    
    let token_data = TokenData {
        total_supply: params.initial_supply,
        decimals: params.decimals,
        name: params.name,
        symbol: params.symbol,
        is_frozen: false,
    };
    
    let balance_data = BalanceData {
        token_id: instruction.target_objects[0],
        owner_id: instruction.target_objects[1],
        amount: params.initial_supply,
    };
    
    let token_object = UnitsObject {
        id: instruction.target_objects[0],
        controller_id: instruction.controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&token_data).unwrap(),
    };
    
    let balance_object = UnitsObject {
        id: instruction.target_objects[1],
        controller_id: instruction.controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&balance_data).unwrap(),
    };
    
    Ok(vec![
        ObjectEffect::creation(token_object),
        ObjectEffect::creation(balance_object),
    ])
}

fn simulate_transfer(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, TokenError> {
    let params: TransferParams = borsh::from_slice(&instruction.params)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    if instruction.target_objects.len() < 3 {
        return Err(TokenError::from_code(TokenError::INVALID_PARAMS));
    }
    
    // Get objects
    let token = context.get_object(&instruction.target_objects[0])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let from_balance = context.get_object(&instruction.target_objects[1])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let to_balance = context.get_object(&instruction.target_objects[2])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    // Parse data
    let token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let mut from_data: BalanceData = borsh::from_slice(&from_balance.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let mut to_data: BalanceData = borsh::from_slice(&to_balance.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    // Validate
    if token_data.is_frozen {
        return Err(TokenError::from_code(TokenError::TOKEN_FROZEN));
    }
    
    if from_data.amount < params.amount {
        return Err(TokenError::from_code(TokenError::INSUFFICIENT_BALANCE));
    }
    
    if to_data.amount.checked_add(params.amount).is_none() {
        return Err(TokenError::from_code(TokenError::OVERFLOW));
    }
    
    // Update balances
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

fn simulate_mint(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, TokenError> {
    let params: MintParams = borsh::from_slice(&instruction.params)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    if instruction.target_objects.len() < 2 {
        return Err(TokenError::from_code(TokenError::INVALID_PARAMS));
    }
    
    let token = context.get_object(&instruction.target_objects[0])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let balance = context.get_object(&instruction.target_objects[1])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let mut balance_data: BalanceData = borsh::from_slice(&balance.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    // Check for overflow
    if token_data.total_supply.checked_add(params.amount).is_none() {
        return Err(TokenError::from_code(TokenError::OVERFLOW));
    }
    if balance_data.amount.checked_add(params.amount).is_none() {
        return Err(TokenError::from_code(TokenError::OVERFLOW));
    }
    
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

fn simulate_burn(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, TokenError> {
    let params: BurnParams = borsh::from_slice(&instruction.params)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    if instruction.target_objects.len() < 2 {
        return Err(TokenError::from_code(TokenError::INVALID_PARAMS));
    }
    
    let token = context.get_object(&instruction.target_objects[0])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let balance = context.get_object(&instruction.target_objects[1])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    let mut balance_data: BalanceData = borsh::from_slice(&balance.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    // Validate
    if balance_data.amount < params.amount {
        return Err(TokenError::from_code(TokenError::INSUFFICIENT_BALANCE));
    }
    if token_data.total_supply < params.amount {
        return Err(TokenError::from_code(TokenError::INVALID_PARAMS));
    }
    
    token_data.total_supply -= params.amount;
    balance_data.amount -= params.amount;
    
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

fn simulate_freeze(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, TokenError> {
    if instruction.target_objects.is_empty() {
        return Err(TokenError::from_code(TokenError::INVALID_PARAMS));
    }
    
    let token = context.get_object(&instruction.target_objects[0])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    token_data.is_frozen = true;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(token.clone(), updated_token)])
}

fn simulate_unfreeze(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, TokenError> {
    if instruction.target_objects.is_empty() {
        return Err(TokenError::from_code(TokenError::INVALID_PARAMS));
    }
    
    let token = context.get_object(&instruction.target_objects[0])
        .ok_or_else(|| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    let mut token_data: TokenData = borsh::from_slice(&token.data)
        .map_err(|_| TokenError::from_code(TokenError::INVALID_PARAMS))?;
    
    token_data.is_frozen = false;
    
    let updated_token = UnitsObject {
        id: token.id,
        controller_id: token.controller_id,
        object_type: token.object_type.clone(),
        data: borsh::to_vec(&token_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(token.clone(), updated_token)])
}

#[test]
fn test_complete_token_lifecycle() {
    let mut context = TestContext::new();
    
    // IDs for our test
    let token_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
    let alice_balance_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
    let bob_balance_id = UnitsObjectId::new([3; OBJECT_ID_SIZE]);
    let _charlie_balance_id = UnitsObjectId::new([4; OBJECT_ID_SIZE]);
    let controller_id = UnitsObjectId::new([5; OBJECT_ID_SIZE]);
    let _alice_id = UnitsObjectId::new([6; OBJECT_ID_SIZE]);
    let bob_id = UnitsObjectId::new([7; OBJECT_ID_SIZE]);
    let _charlie_id = UnitsObjectId::new([8; OBJECT_ID_SIZE]);
    
    // Step 1: Create Token - Create a new token with initial supply to Alice
    println!("Step 1: Creating token...");
    let tokenize_params = TokenizeParams {
        initial_supply: 1_000_000,
        decimals: 18,
        name: "Test Token".to_string(),
        symbol: "TEST".to_string(),
    };
    
    let tokenize_instruction = Instruction {
        controller_id,
        target_function: "create_token".to_string(),
        target_objects: vec![token_id, alice_balance_id],
        params: borsh::to_vec(&tokenize_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&tokenize_instruction, &context)
        .expect("Tokenize should succeed");
    context.apply_effects(effects);
    
    // Verify token was created
    let token = context.get_object(&token_id).expect("Token should exist");
    let token_data: TokenData = borsh::from_slice(&token.data).unwrap();
    assert_eq!(token_data.total_supply, 1_000_000);
    assert_eq!(token_data.symbol, "TEST");
    assert!(!token_data.is_frozen);
    
    // Verify Alice got the initial supply
    let alice_balance = context.get_object(&alice_balance_id).expect("Alice balance should exist");
    let alice_data: BalanceData = borsh::from_slice(&alice_balance.data).unwrap();
    assert_eq!(alice_data.amount, 1_000_000);
    
    // Step 2: Create empty balance for Bob
    println!("Step 2: Creating Bob's balance...");
    context.add_object(UnitsObject {
        id: bob_balance_id,
        controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&BalanceData {
            token_id,
            owner_id: bob_id,
            amount: 0,
        }).unwrap(),
    });
    
    // Step 3: Transfer from Alice to Bob
    println!("Step 3: Transferring tokens from Alice to Bob...");
    let transfer_params = TransferParams { amount: 100_000 };
    
    let transfer_instruction = Instruction {
        controller_id,
        target_function: "transfer_token".to_string(),
        target_objects: vec![token_id, alice_balance_id, bob_balance_id],
        params: borsh::to_vec(&transfer_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&transfer_instruction, &context)
        .expect("Transfer should succeed");
    context.apply_effects(effects);
    
    // Verify balances after transfer
    let alice_balance = context.get_object(&alice_balance_id).unwrap();
    let alice_data: BalanceData = borsh::from_slice(&alice_balance.data).unwrap();
    assert_eq!(alice_data.amount, 900_000);
    
    let bob_balance = context.get_object(&bob_balance_id).unwrap();
    let bob_data: BalanceData = borsh::from_slice(&bob_balance.data).unwrap();
    assert_eq!(bob_data.amount, 100_000);
    
    // Step 4: Mint more tokens to Alice
    println!("Step 4: Minting more tokens to Alice...");
    let mint_params = MintParams { amount: 500_000 };
    
    let mint_instruction = Instruction {
        controller_id,
        target_function: "mint_token".to_string(),
        target_objects: vec![token_id, alice_balance_id],
        params: borsh::to_vec(&mint_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&mint_instruction, &context)
        .expect("Mint should succeed");
    context.apply_effects(effects);
    
    // Verify supply increased
    let token = context.get_object(&token_id).unwrap();
    let token_data: TokenData = borsh::from_slice(&token.data).unwrap();
    assert_eq!(token_data.total_supply, 1_500_000);
    
    let alice_balance = context.get_object(&alice_balance_id).unwrap();
    let alice_data: BalanceData = borsh::from_slice(&alice_balance.data).unwrap();
    assert_eq!(alice_data.amount, 1_400_000);
    
    // Step 5: Freeze the token
    println!("Step 5: Freezing token...");
    let freeze_instruction = Instruction {
        controller_id,
        target_function: "freeze_token".to_string(),
        target_objects: vec![token_id],
        params: vec![],
    };
    
    let effects = simulate_kernel_execution(&freeze_instruction, &context)
        .expect("Freeze should succeed");
    context.apply_effects(effects);
    
    // Step 6: Try to transfer while frozen (should fail)
    println!("Step 6: Attempting transfer while frozen...");
    let transfer_instruction = Instruction {
        controller_id,
        target_function: "transfer_token".to_string(),
        target_objects: vec![token_id, alice_balance_id, bob_balance_id],
        params: borsh::to_vec(&TransferParams { amount: 50_000 }).unwrap(),
    };
    
    let result = simulate_kernel_execution(&transfer_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, TokenError::TOKEN_FROZEN);
    
    // Step 7: Unfreeze the token
    println!("Step 7: Unfreezing token...");
    let unfreeze_instruction = Instruction {
        controller_id,
        target_function: "unfreeze_token".to_string(),
        target_objects: vec![token_id],
        params: vec![],
    };
    
    let effects = simulate_kernel_execution(&unfreeze_instruction, &context)
        .expect("Unfreeze should succeed");
    context.apply_effects(effects);
    
    // Step 8: Burn tokens from Bob
    println!("Step 8: Burning tokens from Bob...");
    let burn_params = BurnParams { amount: 50_000 };
    
    let burn_instruction = Instruction {
        controller_id,
        target_function: "burn_token".to_string(),
        target_objects: vec![token_id, bob_balance_id],
        params: borsh::to_vec(&burn_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&burn_instruction, &context)
        .expect("Burn should succeed");
    context.apply_effects(effects);
    
    // Verify final state
    let token = context.get_object(&token_id).unwrap();
    let token_data: TokenData = borsh::from_slice(&token.data).unwrap();
    assert_eq!(token_data.total_supply, 1_450_000); // 1,500,000 - 50,000
    
    let bob_balance = context.get_object(&bob_balance_id).unwrap();
    let bob_data: BalanceData = borsh::from_slice(&bob_balance.data).unwrap();
    assert_eq!(bob_data.amount, 50_000); // 100,000 - 50,000
    
    println!("All tests passed! Token lifecycle complete.");
}

#[test]
fn test_error_cases() {
    let mut context = TestContext::new();
    
    let token_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
    let alice_balance_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
    let bob_balance_id = UnitsObjectId::new([3; OBJECT_ID_SIZE]);
    let controller_id = UnitsObjectId::new([4; OBJECT_ID_SIZE]);
    
    // Create token and balances
    let token_data = TokenData {
        total_supply: 1000,
        decimals: 18,
        name: "Test".to_string(),
        symbol: "TST".to_string(),
        is_frozen: false,
    };
    
    context.add_object(UnitsObject {
        id: token_id,
        controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&token_data).unwrap(),
    });
    
    context.add_object(UnitsObject {
        id: alice_balance_id,
        controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&BalanceData {
            token_id,
            owner_id: UnitsObjectId::new([5; OBJECT_ID_SIZE]),
            amount: 100,
        }).unwrap(),
    });
    
    context.add_object(UnitsObject {
        id: bob_balance_id,
        controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&BalanceData {
            token_id,
            owner_id: UnitsObjectId::new([6; OBJECT_ID_SIZE]),
            amount: u64::MAX - 50, // Near max for overflow test
        }).unwrap(),
    });
    
    // Test 1: Insufficient balance
    println!("Test 1: Insufficient balance...");
    let transfer_instruction = Instruction {
        controller_id,
        target_function: "transfer_token".to_string(),
        target_objects: vec![token_id, alice_balance_id, bob_balance_id],
        params: borsh::to_vec(&TransferParams { amount: 200 }).unwrap(), // More than Alice has
    };
    
    let result = simulate_kernel_execution(&transfer_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, TokenError::INSUFFICIENT_BALANCE);
    
    // Test 2: Overflow on transfer
    println!("Test 2: Overflow on transfer...");
    let transfer_instruction = Instruction {
        controller_id,
        target_function: "transfer_token".to_string(),
        target_objects: vec![token_id, alice_balance_id, bob_balance_id],
        params: borsh::to_vec(&TransferParams { amount: 100 }).unwrap(),
    };
    
    let result = simulate_kernel_execution(&transfer_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, TokenError::OVERFLOW);
    
    // Test 3: Invalid function
    println!("Test 3: Invalid function...");
    let invalid_instruction = Instruction {
        controller_id,
        target_function: "invalid_function".to_string(),
        target_objects: vec![],
        params: vec![],
    };
    
    let result = simulate_kernel_execution(&invalid_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, TokenError::INVALID_FUNCTION);
    
    // Test 4: Missing objects
    println!("Test 4: Missing objects...");
    let transfer_instruction = Instruction {
        controller_id,
        target_function: "transfer_token".to_string(),
        target_objects: vec![token_id], // Missing balance objects
        params: borsh::to_vec(&TransferParams { amount: 10 }).unwrap(),
    };
    
    let result = simulate_kernel_execution(&transfer_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, TokenError::INVALID_PARAMS);
    
    println!("All error cases handled correctly!");
}