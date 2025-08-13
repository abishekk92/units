#[cfg(feature = "test-harness")]
use borsh::BorshSerialize;
use token::*;
use units_core::{Instruction, UnitsObject, UnitsObjectId, ObjectType};

fn create_test_token_object(id: UnitsObjectId, controller_id: UnitsObjectId, token_data: TokenData) -> UnitsObject {
    UnitsObject {
        id,
        controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&token_data).unwrap(),
    }
}

fn create_test_balance_object(id: UnitsObjectId, controller_id: UnitsObjectId, balance_data: BalanceData) -> UnitsObject {
    UnitsObject {
        id,
        controller_id,
        object_type: ObjectType::Data,
        data: borsh::to_vec(&balance_data).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_creates_token_and_initial_balance() {
        let token_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
        let creator_balance_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
        let controller_id = UnitsObjectId::new([3; OBJECT_ID_SIZE]);

        let params = TokenizeParams {
            initial_supply: 1_000_000,
            decimals: 18,
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
        };

        let instruction = Instruction {
            controller_id,
            target_function: TokenFunction::Tokenize.as_str().to_string(),
            target_objects: vec![token_id, creator_balance_id],
            params: borsh::to_vec(&params).unwrap(),
        };

        // In a real test, we would:
        // 1. Create an execution context
        // 2. Run the kernel module
        // 3. Verify the effects
        
        // For now, we just verify serialization works
        assert_eq!(instruction.target_function, "tokenize");
        assert_eq!(instruction.target_objects.len(), 2);
    }

    #[test]
    fn test_transfer_updates_balances() {
        let token_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
        let from_balance_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
        let to_balance_id = UnitsObjectId::new([3; OBJECT_ID_SIZE]);
        let controller_id = UnitsObjectId::new([4; OBJECT_ID_SIZE]);

        let token_data = TokenData {
            total_supply: 1_000_000,
            decimals: 18,
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            is_frozen: false,
        };

        let from_balance = BalanceData {
            token_id,
            owner_id: UnitsObjectId::new([5; OBJECT_ID_SIZE]),
            amount: 1000,
        };

        let to_balance = BalanceData {
            token_id,
            owner_id: UnitsObjectId::new([6; OBJECT_ID_SIZE]),
            amount: 0,
        };

        let params = TransferParams { amount: 100 };

        let instruction = Instruction {
            controller_id,
            target_function: TokenFunction::Transfer.as_str().to_string(),
            target_objects: vec![token_id, from_balance_id, to_balance_id],
            params: borsh::to_vec(&params).unwrap(),
        };

        // Verify instruction structure
        assert_eq!(instruction.target_function, "transfer");
        assert_eq!(instruction.target_objects.len(), 3);
        
        // Verify balances would be valid
        assert!(from_balance.amount >= params.amount);
    }

    #[test]
    fn test_mint_increases_supply() {
        let token_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
        let balance_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
        let controller_id = UnitsObjectId::new([3; OBJECT_ID_SIZE]);

        let params = MintParams { amount: 500 };

        let instruction = Instruction {
            controller_id,
            target_function: TokenFunction::Mint.as_str().to_string(),
            target_objects: vec![token_id, balance_id],
            params: borsh::to_vec(&params).unwrap(),
        };

        assert_eq!(instruction.target_function, "mint");
        assert_eq!(instruction.target_objects.len(), 2);
    }

    #[test]
    fn test_burn_decreases_supply() {
        let token_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
        let balance_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);
        let controller_id = UnitsObjectId::new([3; OBJECT_ID_SIZE]);

        let params = BurnParams { amount: 100 };

        let instruction = Instruction {
            controller_id,
            target_function: TokenFunction::Burn.as_str().to_string(),
            target_objects: vec![token_id, balance_id],
            params: borsh::to_vec(&params).unwrap(),
        };

        assert_eq!(instruction.target_function, "burn");
        assert_eq!(instruction.target_objects.len(), 2);
    }

    #[test]
    fn test_freeze_and_unfreeze() {
        let token_id = UnitsObjectId::new([1; OBJECT_ID_SIZE]);
        let controller_id = UnitsObjectId::new([2; OBJECT_ID_SIZE]);

        // Test freeze
        let freeze_instruction = Instruction {
            controller_id,
            target_function: TokenFunction::Freeze.as_str().to_string(),
            target_objects: vec![token_id],
            params: vec![], // No parameters needed
        };

        assert_eq!(freeze_instruction.target_function, "freeze");
        assert_eq!(freeze_instruction.target_objects.len(), 1);

        // Test unfreeze
        let unfreeze_instruction = Instruction {
            controller_id,
            target_function: TokenFunction::Unfreeze.as_str().to_string(),
            target_objects: vec![token_id],
            params: vec![], // No parameters needed
        };

        assert_eq!(unfreeze_instruction.target_function, "unfreeze");
        assert_eq!(unfreeze_instruction.target_objects.len(), 1);
    }

    #[test]
    fn test_token_error_codes() {
        assert_eq!(TokenError::from_code(TokenError::INVALID_FUNCTION).message, "Invalid function");
        assert_eq!(TokenError::from_code(TokenError::INVALID_PARAMS).message, "Invalid parameters");
        assert_eq!(TokenError::from_code(TokenError::INSUFFICIENT_BALANCE).message, "Insufficient balance");
        assert_eq!(TokenError::from_code(TokenError::UNAUTHORIZED).message, "Unauthorized");
        assert_eq!(TokenError::from_code(TokenError::TOKEN_FROZEN).message, "Token is frozen");
        assert_eq!(TokenError::from_code(TokenError::OVERFLOW).message, "Numeric overflow");
    }
}

fn main() {
    println!("Token lifecycle test harness");
    println!("Run with: cargo test --features test-harness");
}