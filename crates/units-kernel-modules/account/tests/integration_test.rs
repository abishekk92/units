use std::collections::HashMap;
use account::{
    AccountData, validate_username,
};

#[test]
fn test_username_validation() {
    // Valid usernames
    assert!(validate_username("user123"));
    assert!(validate_username("test_user"));
    assert!(validate_username("ABC"));
    assert!(validate_username("user_name_123"));
    
    // Invalid usernames
    assert!(!validate_username("ab")); // Too short
    assert!(!validate_username(&"a".repeat(33))); // Too long
    assert!(!validate_username("user@123")); // Invalid character
    assert!(!validate_username("user name")); // Space not allowed
    assert!(!validate_username("user-name")); // Dash not allowed
    assert!(!validate_username("user.name")); // Dot not allowed
}

#[test]
fn test_account_data_creation() {
    use units_kernel_sdk::UnitsObjectId;
    
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

#[test]
fn test_account_data_serialization() {
    use units_kernel_sdk::UnitsObjectId;
    
    let account_id = UnitsObjectId::new([1u8; 32]);
    let recovery_id = UnitsObjectId::new([2u8; 32]);
    
    let mut metadata = HashMap::new();
    metadata.insert("email".to_string(), "test@example.com".to_string());
    
    let account = AccountData::new(account_id, 1234567890)
        .with_username("testuser".to_string())
        .with_display_name("Test User".to_string())
        .with_metadata(metadata.clone())
        .with_recovery_addresses(vec![recovery_id]);
    
    // Test borsh serialization
    let serialized = borsh::to_vec(&account).unwrap();
    let deserialized: AccountData = borsh::from_slice(&serialized).unwrap();
    
    assert_eq!(account.account_id, deserialized.account_id);
    assert_eq!(account.username, deserialized.username);
    assert_eq!(account.display_name, deserialized.display_name);
    assert_eq!(account.metadata, deserialized.metadata);
    assert_eq!(account.is_active, deserialized.is_active);
    assert_eq!(account.recovery_addresses, deserialized.recovery_addresses);
    assert_eq!(account.created_at, deserialized.created_at);
    assert_eq!(account.updated_at, deserialized.updated_at);
}