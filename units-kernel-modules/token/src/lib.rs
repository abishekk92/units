#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use borsh::{BorshDeserialize, BorshSerialize};
use units_kernel_sdk::{UnitsObjectId, OBJECT_ID_SIZE};

pub const TOKEN_MODULE_NAME: &str = "token";

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TokenData {
    pub total_supply: u64,
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
    pub is_frozen: bool,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BalanceData {
    pub token_id: UnitsObjectId,
    pub owner_id: UnitsObjectId,
    pub amount: u64,
}

// UnitsObjectId now implements BorshSerialize/Deserialize in the SDK

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TransferParams {
    pub amount: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TokenizeParams {
    pub initial_supply: u64,
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MintParams {
    pub amount: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BurnParams {
    pub amount: u64,
}

pub enum TokenFunction {
    Tokenize,
    Transfer,
    Mint,
    Burn,
    Freeze,
    Unfreeze,
}

impl TokenFunction {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenFunction::Tokenize => "tokenize",
            TokenFunction::Transfer => "transfer",
            TokenFunction::Mint => "mint",
            TokenFunction::Burn => "burn",
            TokenFunction::Freeze => "freeze",
            TokenFunction::Unfreeze => "unfreeze",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenError {
    pub code: i32,
    pub message: String,
}

impl TokenError {
    pub const INVALID_FUNCTION: i32 = -1;
    pub const INVALID_PARAMS: i32 = -2;
    pub const INSUFFICIENT_BALANCE: i32 = -3;
    pub const UNAUTHORIZED: i32 = -4;
    pub const TOKEN_FROZEN: i32 = -5;
    pub const OVERFLOW: i32 = -6;

    pub fn from_code(code: i32) -> Self {
        let message = match code {
            Self::INVALID_FUNCTION => "Invalid function",
            Self::INVALID_PARAMS => "Invalid parameters",
            Self::INSUFFICIENT_BALANCE => "Insufficient balance",
            Self::UNAUTHORIZED => "Unauthorized",
            Self::TOKEN_FROZEN => "Token is frozen",
            Self::OVERFLOW => "Numeric overflow",
            _ => "Unknown error",
        };
        Self {
            code,
            message: message.to_string(),
        }
    }
}

impl core::fmt::Display for TokenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Token error {}: {}", self.code, self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TokenError {}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_token_data_serialization() {
        let token = TokenData {
            total_supply: 1_000_000,
            decimals: 18,
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            is_frozen: false,
        };

        let serialized = borsh::to_vec(&token).unwrap();
        let deserialized: TokenData = borsh::from_slice(&serialized).unwrap();

        assert_eq!(token.total_supply, deserialized.total_supply);
        assert_eq!(token.decimals, deserialized.decimals);
        assert_eq!(token.name, deserialized.name);
        assert_eq!(token.symbol, deserialized.symbol);
        assert_eq!(token.is_frozen, deserialized.is_frozen);
    }

    #[test]
    fn test_balance_data_serialization() {
        let balance = BalanceData {
            token_id: UnitsObjectId::new([1; OBJECT_ID_SIZE]),
            owner_id: UnitsObjectId::new([2; OBJECT_ID_SIZE]),
            amount: 500_000,
        };

        let serialized = borsh::to_vec(&balance).unwrap();
        let deserialized: BalanceData = borsh::from_slice(&serialized).unwrap();

        assert_eq!(balance.token_id, deserialized.token_id);
        assert_eq!(balance.owner_id, deserialized.owner_id);
        assert_eq!(balance.amount, deserialized.amount);
    }

    #[test]
    fn test_transfer_params_serialization() {
        let params = TransferParams { amount: 100 };

        let serialized = borsh::to_vec(&params).unwrap();
        let deserialized: TransferParams = borsh::from_slice(&serialized).unwrap();

        assert_eq!(params.amount, deserialized.amount);
    }

    #[test]
    fn test_tokenize_params_serialization() {
        let params = TokenizeParams {
            initial_supply: 1_000_000,
            decimals: 18,
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
        };

        let serialized = borsh::to_vec(&params).unwrap();
        let deserialized: TokenizeParams = borsh::from_slice(&serialized).unwrap();

        assert_eq!(params.initial_supply, deserialized.initial_supply);
        assert_eq!(params.decimals, deserialized.decimals);
        assert_eq!(params.name, deserialized.name);
        assert_eq!(params.symbol, deserialized.symbol);
    }

    #[test]
    fn test_token_functions() {
        assert_eq!(TokenFunction::Tokenize.as_str(), "tokenize");
        assert_eq!(TokenFunction::Transfer.as_str(), "transfer");
        assert_eq!(TokenFunction::Mint.as_str(), "mint");
        assert_eq!(TokenFunction::Burn.as_str(), "burn");
        assert_eq!(TokenFunction::Freeze.as_str(), "freeze");
        assert_eq!(TokenFunction::Unfreeze.as_str(), "unfreeze");
    }

    #[test]
    fn test_token_error_codes() {
        let err = TokenError::from_code(TokenError::INVALID_FUNCTION);
        assert_eq!(err.code, TokenError::INVALID_FUNCTION);
        assert_eq!(err.message, "Invalid function");

        let err = TokenError::from_code(TokenError::INSUFFICIENT_BALANCE);
        assert_eq!(err.code, TokenError::INSUFFICIENT_BALANCE);
        assert_eq!(err.message, "Insufficient balance");
    }
}