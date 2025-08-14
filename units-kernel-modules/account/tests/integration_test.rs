use borsh::BorshSerialize;
use account::*;
use units_kernel_sdk::{Instruction, UnitsObject, UnitsObjectId, ObjectType, ObjectEffect, OBJECT_ID_SIZE};

/// Helper to create a test execution context
struct TestContext {
    objects: Vec<UnitsObject>,
    current_timestamp: u64,
}

impl TestContext {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            current_timestamp: 1234567890,
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
    
    fn advance_time(&mut self, seconds: u64) {
        self.current_timestamp += seconds;
    }
}

/// Simulates kernel module execution (in real system this would run in RISC-V VM)
fn simulate_kernel_execution(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    match instruction.target_function.as_str() {
        "create_account" => simulate_create_account(instruction, context),
        "update_account" => simulate_update_account(instruction, context),
        "add_recovery_address" => simulate_add_recovery_address(instruction, context),
        "remove_recovery_address" => simulate_remove_recovery_address(instruction, context),
        "deactivate_account" => simulate_deactivate_account(instruction, context),
        "reactivate_account" => simulate_reactivate_account(instruction, context),
        "get_account" => simulate_get_account(instruction, context),
        _ => Err(AccountError::from_code(AccountError::INVALID_FUNCTION)),
    }
}

fn simulate_create_account(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    let params: CreateAccountParams = borsh::from_slice(&instruction.params)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    if instruction.target_objects.is_empty() {
        return Err(AccountError::from_code(AccountError::INVALID_PARAMS));
    }
    
    let account_id = instruction.target_objects[0];
    
    // Check if account already exists
    if context.get_object(&account_id).is_some() {
        return Err(AccountError::from_code(AccountError::ACCOUNT_EXISTS));
    }
    
    // Validate username if provided
    if let Some(ref username) = params.username {
        if !validate_username(username) {
            return Err(AccountError::from_code(AccountError::INVALID_USERNAME));
        }
    }
    
    let account_data = AccountData {
        account_id,
        username: params.username,
        display_name: params.display_name,
        metadata: params.metadata,
        is_active: true,
        created_at: context.current_timestamp,
        updated_at: context.current_timestamp,
        recovery_addresses: params.recovery_addresses,
    };
    
    let account_object = UnitsObject {
        id: account_id,
        controller_id: instruction.controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&account_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::creation(account_object)])
}

fn simulate_update_account(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    let params: UpdateAccountParams = borsh::from_slice(&instruction.params)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    if instruction.target_objects.is_empty() {
        return Err(AccountError::from_code(AccountError::INVALID_PARAMS));
    }
    
    let account_id = instruction.target_objects[0];
    let account = context.get_object(&account_id)
        .ok_or_else(|| AccountError::from_code(AccountError::ACCOUNT_NOT_FOUND))?;
    
    // Verify controller
    if account.controller_id != instruction.controller_id {
        return Err(AccountError::from_code(AccountError::UNAUTHORIZED));
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    // Check if account is active
    if !account_data.is_active {
        return Err(AccountError::from_code(AccountError::ACCOUNT_INACTIVE));
    }
    
    // Update fields if provided
    if let Some(username) = params.username {
        if !validate_username(&username) {
            return Err(AccountError::from_code(AccountError::INVALID_USERNAME));
        }
        account_data.username = Some(username);
    }
    
    if let Some(display_name) = params.display_name {
        account_data.display_name = display_name;
    }
    
    if let Some(metadata) = params.metadata {
        account_data.metadata = metadata;
    }
    
    account_data.updated_at = context.current_timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn simulate_add_recovery_address(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    let params: AddRecoveryAddressParams = borsh::from_slice(&instruction.params)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    if instruction.target_objects.is_empty() {
        return Err(AccountError::from_code(AccountError::INVALID_PARAMS));
    }
    
    let account_id = instruction.target_objects[0];
    let account = context.get_object(&account_id)
        .ok_or_else(|| AccountError::from_code(AccountError::ACCOUNT_NOT_FOUND))?;
    
    // Verify controller
    if account.controller_id != instruction.controller_id {
        return Err(AccountError::from_code(AccountError::UNAUTHORIZED));
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    // Check if recovery address already exists
    if account_data.recovery_addresses.contains(&params.recovery_address) {
        return Err(AccountError::from_code(AccountError::RECOVERY_ADDRESS_EXISTS));
    }
    
    account_data.recovery_addresses.push(params.recovery_address);
    account_data.updated_at = context.current_timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn simulate_remove_recovery_address(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    let params: RemoveRecoveryAddressParams = borsh::from_slice(&instruction.params)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    if instruction.target_objects.is_empty() {
        return Err(AccountError::from_code(AccountError::INVALID_PARAMS));
    }
    
    let account_id = instruction.target_objects[0];
    let account = context.get_object(&account_id)
        .ok_or_else(|| AccountError::from_code(AccountError::ACCOUNT_NOT_FOUND))?;
    
    // Verify controller
    if account.controller_id != instruction.controller_id {
        return Err(AccountError::from_code(AccountError::UNAUTHORIZED));
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    // Find and remove the recovery address
    let initial_len = account_data.recovery_addresses.len();
    account_data.recovery_addresses.retain(|addr| addr != &params.recovery_address);
    
    if account_data.recovery_addresses.len() == initial_len {
        return Err(AccountError::from_code(AccountError::RECOVERY_ADDRESS_NOT_FOUND));
    }
    
    account_data.updated_at = context.current_timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn simulate_deactivate_account(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    let _params: DeactivateAccountParams = borsh::from_slice(&instruction.params)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    if instruction.target_objects.is_empty() {
        return Err(AccountError::from_code(AccountError::INVALID_PARAMS));
    }
    
    let account_id = instruction.target_objects[0];
    let account = context.get_object(&account_id)
        .ok_or_else(|| AccountError::from_code(AccountError::ACCOUNT_NOT_FOUND))?;
    
    // Verify controller
    if account.controller_id != instruction.controller_id {
        return Err(AccountError::from_code(AccountError::UNAUTHORIZED));
    }
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    if !account_data.is_active {
        return Err(AccountError::from_code(AccountError::ACCOUNT_INACTIVE));
    }
    
    account_data.is_active = false;
    account_data.updated_at = context.current_timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn simulate_reactivate_account(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    let _params: ReactivateAccountParams = borsh::from_slice(&instruction.params)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    if instruction.target_objects.is_empty() {
        return Err(AccountError::from_code(AccountError::INVALID_PARAMS));
    }
    
    let account_id = instruction.target_objects[0];
    let account = context.get_object(&account_id)
        .ok_or_else(|| AccountError::from_code(AccountError::ACCOUNT_NOT_FOUND))?;
    
    let mut account_data: AccountData = borsh::from_slice(&account.data)
        .map_err(|_| AccountError::from_code(AccountError::INVALID_PARAMS))?;
    
    // Verify controller or recovery address
    let is_authorized = account.controller_id == instruction.controller_id ||
        account_data.recovery_addresses.contains(&instruction.controller_id);
    
    if !is_authorized {
        return Err(AccountError::from_code(AccountError::UNAUTHORIZED));
    }
    
    if account_data.is_active {
        return Err(AccountError::from_code(AccountError::ACCOUNT_INACTIVE)); // Already active
    }
    
    account_data.is_active = true;
    account_data.updated_at = context.current_timestamp;
    
    let updated_account = UnitsObject {
        id: account.id,
        controller_id: account.controller_id,
        object_type: account.object_type.clone(),
        data: borsh::to_vec(&account_data).unwrap(),
    };
    
    Ok(vec![ObjectEffect::modification(account.clone(), updated_account)])
}

fn simulate_get_account(
    instruction: &Instruction,
    context: &TestContext,
) -> Result<Vec<ObjectEffect>, AccountError> {
    if instruction.target_objects.is_empty() {
        return Err(AccountError::from_code(AccountError::INVALID_PARAMS));
    }
    
    let account_id = instruction.target_objects[0];
    let _account = context.get_object(&account_id)
        .ok_or_else(|| AccountError::from_code(AccountError::ACCOUNT_NOT_FOUND))?;
    
    // Read-only operation, return empty effects
    Ok(vec![])
}

#[test]
fn test_complete_account_lifecycle() {
    let mut context = TestContext::new();
    
    // IDs for our test
    let alice_account_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
    let alice_controller_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
    let recovery_id_1 = UnitsObjectId::new([3; OBJECT_ID_SIZE]);
    let recovery_id_2 = UnitsObjectId::new([4; OBJECT_ID_SIZE]);
    
    // Step 1: Create Account
    println!("Step 1: Creating account...");
    let create_params = CreateAccountParams {
        username: Some("alice123".to_string()),
        display_name: "Alice Smith".to_string(),
        metadata: vec![
            AccountMetadata {
                key: "email".to_string(),
                value: "alice@example.com".to_string(),
            },
            AccountMetadata {
                key: "bio".to_string(),
                value: "Software developer and crypto enthusiast".to_string(),
            },
        ],
        recovery_addresses: vec![recovery_id_1],
    };
    
    let create_instruction = Instruction {
        controller_id: alice_controller_id,
        target_function: "create_account".to_string(),
        target_objects: vec![alice_account_id],
        params: borsh::to_vec(&create_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&create_instruction, &context)
        .expect("Create account should succeed");
    context.apply_effects(effects);
    
    // Verify account was created
    let account = context.get_object(&alice_account_id).expect("Account should exist");
    let account_data: AccountData = borsh::from_slice(&account.data).unwrap();
    assert_eq!(account_data.username, Some("alice123".to_string()));
    assert_eq!(account_data.display_name, "Alice Smith");
    assert!(account_data.is_active);
    assert_eq!(account_data.recovery_addresses.len(), 1);
    assert_eq!(account_data.metadata.len(), 2);
    
    // Step 2: Update Account
    println!("Step 2: Updating account...");
    context.advance_time(3600); // 1 hour later
    
    let update_params = UpdateAccountParams {
        username: None, // Keep existing username
        display_name: Some("Alice Johnson".to_string()),
        metadata: Some(vec![
            AccountMetadata {
                key: "email".to_string(),
                value: "alice.johnson@example.com".to_string(),
            },
            AccountMetadata {
                key: "bio".to_string(),
                value: "Senior software developer".to_string(),
            },
            AccountMetadata {
                key: "location".to_string(),
                value: "San Francisco, CA".to_string(),
            },
        ]),
    };
    
    let update_instruction = Instruction {
        controller_id: alice_controller_id,
        target_function: "update_account".to_string(),
        target_objects: vec![alice_account_id],
        params: borsh::to_vec(&update_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&update_instruction, &context)
        .expect("Update account should succeed");
    context.apply_effects(effects);
    
    // Verify updates
    let account = context.get_object(&alice_account_id).unwrap();
    let account_data: AccountData = borsh::from_slice(&account.data).unwrap();
    assert_eq!(account_data.display_name, "Alice Johnson");
    assert_eq!(account_data.metadata.len(), 3);
    assert!(account_data.updated_at > account_data.created_at);
    
    // Step 3: Add Recovery Address
    println!("Step 3: Adding recovery address...");
    let add_recovery_params = AddRecoveryAddressParams {
        recovery_address: recovery_id_2,
    };
    
    let add_recovery_instruction = Instruction {
        controller_id: alice_controller_id,
        target_function: "add_recovery_address".to_string(),
        target_objects: vec![alice_account_id],
        params: borsh::to_vec(&add_recovery_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&add_recovery_instruction, &context)
        .expect("Add recovery address should succeed");
    context.apply_effects(effects);
    
    // Verify recovery address added
    let account = context.get_object(&alice_account_id).unwrap();
    let account_data: AccountData = borsh::from_slice(&account.data).unwrap();
    assert_eq!(account_data.recovery_addresses.len(), 2);
    assert!(account_data.recovery_addresses.contains(&recovery_id_2));
    
    // Step 4: Try to add duplicate recovery address (should fail)
    println!("Step 4: Attempting to add duplicate recovery address...");
    let result = simulate_kernel_execution(&add_recovery_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AccountError::RECOVERY_ADDRESS_EXISTS);
    
    // Step 5: Deactivate Account
    println!("Step 5: Deactivating account...");
    let deactivate_params = DeactivateAccountParams {
        reason: Some("User requested deactivation".to_string()),
    };
    
    let deactivate_instruction = Instruction {
        controller_id: alice_controller_id,
        target_function: "deactivate_account".to_string(),
        target_objects: vec![alice_account_id],
        params: borsh::to_vec(&deactivate_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&deactivate_instruction, &context)
        .expect("Deactivate account should succeed");
    context.apply_effects(effects);
    
    // Verify account is inactive
    let account = context.get_object(&alice_account_id).unwrap();
    let account_data: AccountData = borsh::from_slice(&account.data).unwrap();
    assert!(!account_data.is_active);
    
    // Step 6: Try to update inactive account (should fail)
    println!("Step 6: Attempting to update inactive account...");
    let update_instruction = Instruction {
        controller_id: alice_controller_id,
        target_function: "update_account".to_string(),
        target_objects: vec![alice_account_id],
        params: borsh::to_vec(&UpdateAccountParams {
            username: None,
            display_name: Some("Should Fail".to_string()),
            metadata: None,
        }).unwrap(),
    };
    
    let result = simulate_kernel_execution(&update_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AccountError::ACCOUNT_INACTIVE);
    
    // Step 7: Reactivate Account using recovery address
    println!("Step 7: Reactivating account with recovery address...");
    let reactivate_params = ReactivateAccountParams {
        verification_data: None,
    };
    
    let reactivate_instruction = Instruction {
        controller_id: recovery_id_1, // Using recovery address
        target_function: "reactivate_account".to_string(),
        target_objects: vec![alice_account_id],
        params: borsh::to_vec(&reactivate_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&reactivate_instruction, &context)
        .expect("Reactivate account should succeed");
    context.apply_effects(effects);
    
    // Verify account is active again
    let account = context.get_object(&alice_account_id).unwrap();
    let account_data: AccountData = borsh::from_slice(&account.data).unwrap();
    assert!(account_data.is_active);
    
    // Step 8: Remove Recovery Address
    println!("Step 8: Removing recovery address...");
    let remove_recovery_params = RemoveRecoveryAddressParams {
        recovery_address: recovery_id_1,
    };
    
    let remove_recovery_instruction = Instruction {
        controller_id: alice_controller_id,
        target_function: "remove_recovery_address".to_string(),
        target_objects: vec![alice_account_id],
        params: borsh::to_vec(&remove_recovery_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&remove_recovery_instruction, &context)
        .expect("Remove recovery address should succeed");
    context.apply_effects(effects);
    
    // Verify recovery address removed
    let account = context.get_object(&alice_account_id).unwrap();
    let account_data: AccountData = borsh::from_slice(&account.data).unwrap();
    assert_eq!(account_data.recovery_addresses.len(), 1);
    assert!(!account_data.recovery_addresses.contains(&recovery_id_1));
    
    println!("All tests passed! Account lifecycle complete.");
}

#[test]
fn test_error_cases() {
    let mut context = TestContext::new();
    
    let account_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
    let controller_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
    let unauthorized_id = UnitsObjectId::new([3; OBJECT_ID_SIZE]);
    
    // Test 1: Invalid username
    println!("Test 1: Invalid username...");
    let create_params = CreateAccountParams {
        username: Some("a".to_string()), // Too short
        display_name: "Test".to_string(),
        metadata: vec![],
        recovery_addresses: vec![],
    };
    
    let create_instruction = Instruction {
        controller_id,
        target_function: "create_account".to_string(),
        target_objects: vec![account_id],
        params: borsh::to_vec(&create_params).unwrap(),
    };
    
    let result = simulate_kernel_execution(&create_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AccountError::INVALID_USERNAME);
    
    // Create valid account for further tests
    let create_params = CreateAccountParams {
        username: Some("testuser".to_string()),
        display_name: "Test User".to_string(),
        metadata: vec![],
        recovery_addresses: vec![],
    };
    
    let create_instruction = Instruction {
        controller_id,
        target_function: "create_account".to_string(),
        target_objects: vec![account_id],
        params: borsh::to_vec(&create_params).unwrap(),
    };
    
    let effects = simulate_kernel_execution(&create_instruction, &context).unwrap();
    context.apply_effects(effects);
    
    // Test 2: Duplicate account creation
    println!("Test 2: Duplicate account creation...");
    let result = simulate_kernel_execution(&create_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AccountError::ACCOUNT_EXISTS);
    
    // Test 3: Unauthorized update
    println!("Test 3: Unauthorized update...");
    let update_instruction = Instruction {
        controller_id: unauthorized_id,
        target_function: "update_account".to_string(),
        target_objects: vec![account_id],
        params: borsh::to_vec(&UpdateAccountParams {
            username: None,
            display_name: Some("Hacked".to_string()),
            metadata: None,
        }).unwrap(),
    };
    
    let result = simulate_kernel_execution(&update_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AccountError::UNAUTHORIZED);
    
    // Test 4: Invalid function
    println!("Test 4: Invalid function...");
    let invalid_instruction = Instruction {
        controller_id,
        target_function: "invalid_function".to_string(),
        target_objects: vec![],
        params: vec![],
    };
    
    let result = simulate_kernel_execution(&invalid_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AccountError::INVALID_FUNCTION);
    
    // Test 5: Missing account
    println!("Test 5: Missing account...");
    let missing_id = UnitsObjectId::new([99; OBJECT_ID_SIZE]);
    let get_instruction = Instruction {
        controller_id,
        target_function: "get_account".to_string(),
        target_objects: vec![missing_id],
        params: vec![],
    };
    
    let result = simulate_kernel_execution(&get_instruction, &context);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AccountError::ACCOUNT_NOT_FOUND);
    
    println!("All error cases handled correctly!");
}

#[test]
fn test_username_edge_cases() {
    // Valid usernames
    assert!(validate_username("abc"));
    assert!(validate_username("user123"));
    assert!(validate_username("test_user"));
    assert!(validate_username("_underscore"));
    assert!(validate_username("UPPERCASE"));
    assert!(validate_username("mixedCase123"));
    assert!(validate_username("a".repeat(32).as_str())); // Max length
    
    // Invalid usernames
    assert!(!validate_username("ab")); // Too short
    assert!(!validate_username("a".repeat(33).as_str())); // Too long
    assert!(!validate_username("user-name")); // Hyphen
    assert!(!validate_username("user name")); // Space
    assert!(!validate_username("user@email")); // Special char
    assert!(!validate_username("user.name")); // Dot
    assert!(!validate_username("")); // Empty
}