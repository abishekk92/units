#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as HashMap;

#[cfg(feature = "std")]
use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use units_kernel_sdk::UnitsObjectId;
use crate::crypto::Signature;
use crate::auth::AuthCredential;

pub const MODULE_NAME: &str = "account";
pub const MODULE_VERSION: &str = "0.1.0";

// Module declarations
pub mod module;
pub mod crypto;
pub mod auth;
pub mod enhanced_module;

// Re-export modules for testing
#[cfg(feature = "std")]
pub use crate::module::AccountModule;
#[cfg(feature = "std")]
pub use crate::enhanced_module::EnhancedAccountModule;

// Account data structure
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct AccountData {
    pub account_id: UnitsObjectId,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: HashMap<String, String>,
    pub is_active: bool,
    pub recovery_addresses: Vec<UnitsObjectId>,
    pub created_at: u64,
    pub updated_at: u64,
}

// Account metadata helper
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct AccountMetadata {
    pub entries: HashMap<String, String>,
}

// Function names (Legacy)
pub const FN_CREATE_ACCOUNT: &str = "create_account";
pub const FN_UPDATE_ACCOUNT: &str = "update_account";
pub const FN_ADD_RECOVERY_ADDRESS: &str = "add_recovery_address";
pub const FN_REMOVE_RECOVERY_ADDRESS: &str = "remove_recovery_address";
pub const FN_DEACTIVATE_ACCOUNT: &str = "deactivate_account";
pub const FN_REACTIVATE_ACCOUNT: &str = "reactivate_account";
pub const FN_GET_ACCOUNT: &str = "get_account";

// Flexible Authentication Function names
pub const FN_FLEX_CREATE_ACCOUNT: &str = "flex_create_account";
pub const FN_FLEX_UPDATE_ACCOUNT: &str = "flex_update_account";
pub const FN_FLEX_ADD_RECOVERY_ADDRESS: &str = "flex_add_recovery_address";
pub const FN_FLEX_REMOVE_RECOVERY_ADDRESS: &str = "flex_remove_recovery_address";
pub const FN_FLEX_DEACTIVATE_ACCOUNT: &str = "flex_deactivate_account";
pub const FN_FLEX_REACTIVATE_ACCOUNT: &str = "flex_reactivate_account";

// Error codes
pub const ERROR_INVALID_USERNAME: u32 = 1001;
pub const ERROR_ACCOUNT_NOT_FOUND: u32 = 1002;
pub const ERROR_UNAUTHORIZED: u32 = 1003;
pub const ERROR_ACCOUNT_INACTIVE: u32 = 1004;
pub const ERROR_RECOVERY_ADDRESS_EXISTS: u32 = 1005;
pub const ERROR_RECOVERY_ADDRESS_NOT_FOUND: u32 = 1006;
pub const ERROR_INVALID_RECOVERY_ADDRESS: u32 = 1007;
pub const ERROR_ACCOUNT_ALREADY_ACTIVE: u32 = 1008;
pub const ERROR_SERIALIZATION_FAILED: u32 = 1009;
pub const ERROR_SIGNATURE_VERIFICATION_FAILED: u32 = 1010;
pub const ERROR_INVALID_SIGNATURE: u32 = 1011;
pub const ERROR_MISSING_SIGNATURE: u32 = 1012;

// Parameter structures for each function
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CreateAccountParams {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub recovery_addresses: Option<Vec<UnitsObjectId>>,
    pub signature: Option<Signature>, // Optional for account creation
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct UpdateAccountParams {
    pub account_id: UnitsObjectId,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub signature: Signature, // Required for account updates
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct AddRecoveryAddressParams {
    pub account_id: UnitsObjectId,
    pub recovery_address: UnitsObjectId,
    pub signature: Signature, // Required for security operations
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RemoveRecoveryAddressParams {
    pub account_id: UnitsObjectId,
    pub recovery_address: UnitsObjectId,
    pub signature: Signature, // Required for security operations
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct DeactivateAccountParams {
    pub account_id: UnitsObjectId,
    pub signature: Signature, // Required for deactivation
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ReactivateAccountParams {
    pub account_id: UnitsObjectId,
    pub signature: Signature, // Required for reactivation
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GetAccountParams {
    pub account_id: UnitsObjectId,
}

// ============================================================================
// NEW FLEXIBLE AUTHENTICATION PARAMETER STRUCTURES
// ============================================================================

/// Enhanced parameter structures that support flexible authentication
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlexCreateAccountParams {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub recovery_addresses: Option<Vec<UnitsObjectId>>,
    /// Multiple authentication credentials (signatures, MFA codes, etc.)
    pub credentials: Vec<AuthCredential>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlexUpdateAccountParams {
    pub account_id: UnitsObjectId,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    /// Multiple authentication credentials (signatures, MFA codes, etc.)
    pub credentials: Vec<AuthCredential>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlexAddRecoveryAddressParams {
    pub account_id: UnitsObjectId,
    pub recovery_address: UnitsObjectId,
    /// Multiple authentication credentials (signatures, MFA codes, etc.)
    pub credentials: Vec<AuthCredential>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlexRemoveRecoveryAddressParams {
    pub account_id: UnitsObjectId,
    pub recovery_address: UnitsObjectId,
    /// Multiple authentication credentials (signatures, MFA codes, etc.)
    pub credentials: Vec<AuthCredential>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlexDeactivateAccountParams {
    pub account_id: UnitsObjectId,
    /// Multiple authentication credentials (signatures, MFA codes, etc.)
    pub credentials: Vec<AuthCredential>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlexReactivateAccountParams {
    pub account_id: UnitsObjectId,
    /// Multiple authentication credentials (signatures, MFA codes, etc.)
    pub credentials: Vec<AuthCredential>,
}

// Enhanced account data with authentication policy support
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct EnhancedAccountData {
    pub account_id: UnitsObjectId,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: HashMap<String, String>,
    pub is_active: bool,
    pub recovery_addresses: Vec<UnitsObjectId>,
    pub created_at: u64,
    pub updated_at: u64,
    /// Custom authentication policy for this account (serialized)
    pub auth_policy: Option<Vec<u8>>,
    /// Supported authentication factors for this account
    pub supported_auth_factors: Vec<crate::auth::AuthFactor>,
}

impl EnhancedAccountData {
    pub fn new(account_id: UnitsObjectId, created_at: u64) -> Self {
        Self {
            account_id,
            username: None,
            display_name: None,
            metadata: HashMap::new(),
            is_active: true,
            recovery_addresses: Vec::new(),
            created_at,
            updated_at: created_at,
            auth_policy: None,
            supported_auth_factors: vec![crate::auth::AuthFactor::Signature(crate::auth::SignatureType::Ed25519)],
        }
    }
    
    pub fn with_auth_policy(mut self, policy_bytes: Vec<u8>) -> Self {
        self.auth_policy = Some(policy_bytes);
        self
    }
    
    pub fn with_supported_factors(mut self, factors: Vec<crate::auth::AuthFactor>) -> Self {
        self.supported_auth_factors = factors;
        self
    }
    
    // Include all the existing builder methods
    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }
    
    pub fn with_display_name(mut self, display_name: String) -> Self {
        self.display_name = Some(display_name);
        self
    }
    
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
    
    pub fn with_recovery_addresses(mut self, addresses: Vec<UnitsObjectId>) -> Self {
        self.recovery_addresses = addresses;
        self
    }
}

// Validation functions
pub fn validate_username(username: &str) -> bool {
    let len = username.len();
    if len < 3 || len > 32 {
        return false;
    }
    
    username.chars().all(|c| c.is_alphanumeric() || c == '_')
}

// Helper functions
impl AccountData {
    pub fn new(account_id: UnitsObjectId, created_at: u64) -> Self {
        Self {
            account_id,
            username: None,
            display_name: None,
            metadata: HashMap::new(),
            is_active: true,
            recovery_addresses: Vec::new(),
            created_at,
            updated_at: created_at,
        }
    }
    
    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }
    
    pub fn with_display_name(mut self, display_name: String) -> Self {
        self.display_name = Some(display_name);
        self
    }
    
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
    
    pub fn with_recovery_addresses(mut self, addresses: Vec<UnitsObjectId>) -> Self {
        self.recovery_addresses = addresses;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_username() {
        assert!(validate_username("user123"));
        assert!(validate_username("test_user"));
        assert!(validate_username("ABC"));
        
        assert!(!validate_username("ab")); // Too short
        assert!(!validate_username("a".repeat(33).as_str())); // Too long
        assert!(!validate_username("user@123")); // Invalid character
        assert!(!validate_username("user name")); // Space not allowed
    }
    
    #[test]
    fn test_account_data_builder() {
        let account_id = UnitsObjectId::new([1u8; 32]);
        let timestamp = 1234567890;
        
        let account = AccountData::new(account_id, timestamp)
            .with_username("testuser".to_string())
            .with_display_name("Test User".to_string());
        
        assert_eq!(account.account_id, account_id);
        assert_eq!(account.username, Some("testuser".to_string()));
        assert_eq!(account.display_name, Some("Test User".to_string()));
        assert!(account.is_active);
        assert_eq!(account.created_at, timestamp);
        assert_eq!(account.updated_at, timestamp);
    }
}