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
    CreateToken,
    TransferToken,
    MintToken,
    BurnToken,
    FreezeToken,
    UnfreezeToken,
}

impl TokenFunction {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenFunction::CreateToken => "create_token",
            TokenFunction::TransferToken => "transfer_token",
            TokenFunction::MintToken => "mint_token",
            TokenFunction::BurnToken => "burn_token",
            TokenFunction::FreezeToken => "freeze_token",
            TokenFunction::UnfreezeToken => "unfreeze_token",
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
        assert_eq!(TokenFunction::CreateToken.as_str(), "create_token");
        assert_eq!(TokenFunction::TransferToken.as_str(), "transfer_token");
        assert_eq!(TokenFunction::MintToken.as_str(), "mint_token");
        assert_eq!(TokenFunction::BurnToken.as_str(), "burn_token");
        assert_eq!(TokenFunction::FreezeToken.as_str(), "freeze_token");
        assert_eq!(TokenFunction::UnfreezeToken.as_str(), "unfreeze_token");
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