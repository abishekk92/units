use account::{
    AccountData, validate_username,
    crypto::{Signature, PublicKey, create_operation_message},
};
use units_kernel_sdk::UnitsObjectId;
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, scalar::Scalar};

#[test]
fn test_signature_creation_and_verification() {
    // Test signature structure
    let sig_bytes = [42u8; 64];
    let signature = Signature::new(sig_bytes);
    assert_eq!(signature.to_bytes(), sig_bytes);
    
    // Test from slice
    let sig_from_slice = Signature::from_slice(&sig_bytes).unwrap();
    assert_eq!(sig_from_slice, signature);
    
    // Test invalid length
    let invalid_slice = [1u8; 32];
    assert!(Signature::from_slice(&invalid_slice).is_err());
}

#[test]
fn test_public_key_operations() {
    // Generate a valid Ed25519 point
    let secret = Scalar::from_bytes_mod_order([1u8; 32]);
    let point = secret * ED25519_BASEPOINT_POINT;
    let compressed = point.compress();
    
    // Test conversion from bytes
    let public_key = PublicKey::from_bytes(&compressed.to_bytes()).unwrap();
    assert_eq!(public_key.compressed, compressed);
    assert_eq!(public_key.point, point);
    
    // Test conversion to/from UnitsObjectId
    let object_id = public_key.to_units_object_id();
    let recovered_key = PublicKey::from_units_object_id(&object_id).unwrap();
    assert_eq!(recovered_key.to_bytes(), public_key.to_bytes());
}

#[test]
fn test_operation_message_creation() {
    let account_id = UnitsObjectId::new([42u8; 32]);
    let timestamp = 1234567890u64;
    let params = vec![1, 2, 3, 4];
    
    let message = create_operation_message("update_account", &account_id, timestamp, &params);
    
    // Should contain operation name, account ID, timestamp, and params
    let mut expected = Vec::new();
    expected.extend_from_slice(b"update_account");
    expected.extend_from_slice(account_id.bytes());
    expected.extend_from_slice(&timestamp.to_le_bytes());
    expected.extend_from_slice(&params);
    
    assert_eq!(message, expected);
}

#[test]
fn test_username_validation_still_works() {
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
fn test_account_data_with_new_structure() {
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
    
    // Test serialization still works
    let serialized = borsh::to_vec(&account).unwrap();
    let deserialized: AccountData = borsh::from_slice(&serialized).unwrap();
    
    assert_eq!(account.account_id, deserialized.account_id);
    assert_eq!(account.username, deserialized.username);
    assert_eq!(account.display_name, deserialized.display_name);
}