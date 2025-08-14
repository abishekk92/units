#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use borsh::{BorshDeserialize, BorshSerialize};
use units_kernel_sdk::UnitsObjectId;

pub const ACCOUNT_MODULE_NAME: &str = "account";

/// Account data structure representing a user account
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct AccountData {
    /// The account ID (same as the object ID)
    pub account_id: UnitsObjectId,
    /// Human-readable username (optional)
    pub username: Option<String>,
    /// Display name for the account
    pub display_name: String,
    /// Account metadata (e.g., email, profile picture URL, etc.)
    pub metadata: Vec<AccountMetadata>,
    /// Whether the account is active
    pub is_active: bool,
    /// Account creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
    /// Account recovery addresses (other UnitsObjectIds that can recover this account)
    pub recovery_addresses: Vec<UnitsObjectId>,
}

/// Metadata key-value pairs for account information
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct AccountMetadata {
    pub key: String,
    pub value: String,
}

/// Parameters for creating a new account
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CreateAccountParams {
    pub username: Option<String>,
    pub display_name: String,
    pub metadata: Vec<AccountMetadata>,
    pub recovery_addresses: Vec<UnitsObjectId>,
}

/// Parameters for updating account information
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct UpdateAccountParams {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<Vec<AccountMetadata>>,
}

/// Parameters for adding recovery addresses
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct AddRecoveryAddressParams {
    pub recovery_address: UnitsObjectId,
}

/// Parameters for removing recovery addresses
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RemoveRecoveryAddressParams {
    pub recovery_address: UnitsObjectId,
}

/// Parameters for deactivating an account
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct DeactivateAccountParams {
    pub reason: Option<String>,
}

/// Parameters for reactivating an account
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ReactivateAccountParams {
    pub verification_data: Option<Vec<u8>>,
}

/// Account module functions
pub enum AccountFunction {
    CreateAccount,
    UpdateAccount,
    AddRecoveryAddress,
    RemoveRecoveryAddress,
    DeactivateAccount,
    ReactivateAccount,
    GetAccount,
}

impl AccountFunction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountFunction::CreateAccount => "create_account",
            AccountFunction::UpdateAccount => "update_account",
            AccountFunction::AddRecoveryAddress => "add_recovery_address",
            AccountFunction::RemoveRecoveryAddress => "remove_recovery_address",
            AccountFunction::DeactivateAccount => "deactivate_account",
            AccountFunction::ReactivateAccount => "reactivate_account",
            AccountFunction::GetAccount => "get_account",
        }
    }
}

/// Account error types
#[derive(Debug, Clone)]
pub struct AccountError {
    pub code: i32,
    pub message: String,
}

impl AccountError {
    pub const INVALID_FUNCTION: i32 = -1;
    pub const INVALID_PARAMS: i32 = -2;
    pub const ACCOUNT_EXISTS: i32 = -3;
    pub const ACCOUNT_NOT_FOUND: i32 = -4;
    pub const UNAUTHORIZED: i32 = -5;
    pub const ACCOUNT_INACTIVE: i32 = -6;
    pub const INVALID_USERNAME: i32 = -7;
    pub const USERNAME_TAKEN: i32 = -8;
    pub const INVALID_METADATA: i32 = -9;
    pub const RECOVERY_ADDRESS_EXISTS: i32 = -10;
    pub const RECOVERY_ADDRESS_NOT_FOUND: i32 = -11;

    pub fn from_code(code: i32) -> Self {
        let message = match code {
            Self::INVALID_FUNCTION => "Invalid function",
            Self::INVALID_PARAMS => "Invalid parameters",
            Self::ACCOUNT_EXISTS => "Account already exists",
            Self::ACCOUNT_NOT_FOUND => "Account not found",
            Self::UNAUTHORIZED => "Unauthorized",
            Self::ACCOUNT_INACTIVE => "Account is inactive",
            Self::INVALID_USERNAME => "Invalid username format",
            Self::USERNAME_TAKEN => "Username already taken",
            Self::INVALID_METADATA => "Invalid metadata",
            Self::RECOVERY_ADDRESS_EXISTS => "Recovery address already exists",
            Self::RECOVERY_ADDRESS_NOT_FOUND => "Recovery address not found",
            _ => "Unknown error",
        };
        Self {
            code,
            message: message.to_string(),
        }
    }
}

impl core::fmt::Display for AccountError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Account error {}: {}", self.code, self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccountError {}

/// Helper function to validate username format
pub fn validate_username(username: &str) -> bool {
    // Username must be 3-32 characters, alphanumeric with underscores
    if username.len() < 3 || username.len() > 32 {
        return false;
    }
    
    username.chars().all(|c| c.is_alphanumeric() || c == '_')
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_account_data_serialization() {
        let account = AccountData {
            account_id: UnitsObjectId::new([1; 32]),
            username: Some("alice".to_string()),
            display_name: "Alice Smith".to_string(),
            metadata: vec![
                AccountMetadata {
                    key: "email".to_string(),
                    value: "alice@example.com".to_string(),
                },
            ],
            is_active: true,
            created_at: 1234567890,
            updated_at: 1234567890,
            recovery_addresses: vec![UnitsObjectId::new([2; 32])],
        };

        let serialized = borsh::to_vec(&account).unwrap();
        let deserialized: AccountData = borsh::from_slice(&serialized).unwrap();

        assert_eq!(account.account_id, deserialized.account_id);
        assert_eq!(account.username, deserialized.username);
        assert_eq!(account.display_name, deserialized.display_name);
        assert_eq!(account.metadata.len(), deserialized.metadata.len());
        assert_eq!(account.is_active, deserialized.is_active);
        assert_eq!(account.created_at, deserialized.created_at);
        assert_eq!(account.recovery_addresses.len(), deserialized.recovery_addresses.len());
    }

    #[test]
    fn test_username_validation() {
        assert!(validate_username("alice"));
        assert!(validate_username("bob_123"));
        assert!(validate_username("user_name_123"));
        
        assert!(!validate_username("ab")); // Too short
        assert!(!validate_username("a".repeat(33).as_str())); // Too long
        assert!(!validate_username("user-name")); // Invalid character
        assert!(!validate_username("user name")); // Space not allowed
        assert!(!validate_username("user@name")); // @ not allowed
    }

    #[test]
    fn test_account_functions() {
        assert_eq!(AccountFunction::CreateAccount.as_str(), "create_account");
        assert_eq!(AccountFunction::UpdateAccount.as_str(), "update_account");
        assert_eq!(AccountFunction::AddRecoveryAddress.as_str(), "add_recovery_address");
        assert_eq!(AccountFunction::RemoveRecoveryAddress.as_str(), "remove_recovery_address");
        assert_eq!(AccountFunction::DeactivateAccount.as_str(), "deactivate_account");
        assert_eq!(AccountFunction::ReactivateAccount.as_str(), "reactivate_account");
        assert_eq!(AccountFunction::GetAccount.as_str(), "get_account");
    }

    #[test]
    fn test_account_error_codes() {
        let err = AccountError::from_code(AccountError::INVALID_USERNAME);
        assert_eq!(err.code, AccountError::INVALID_USERNAME);
        assert_eq!(err.message, "Invalid username format");

        let err = AccountError::from_code(AccountError::ACCOUNT_NOT_FOUND);
        assert_eq!(err.code, AccountError::ACCOUNT_NOT_FOUND);
        assert_eq!(err.message, "Account not found");
    }
}