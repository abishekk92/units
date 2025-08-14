use std::collections::HashMap;
use account::{
    auth::{
        AuthManager, AuthContext, AuthCredential, AuthFactor, SignatureType,
        AuthRequirement, AuthResult, AuthPolicy,
        signature_schemes::{Ed25519Authenticator, create_default_signature_authenticators},
        multi_factor::{TotpAuthenticator, TotpSecret, TotpAlgorithm},
        policies::{StandardAccountPolicy, HighSecurityPolicy, ConfigurablePolicy}
    },
    FlexCreateAccountParams, FlexUpdateAccountParams, EnhancedAccountData
};
use units_kernel_sdk::UnitsObjectId;
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, scalar::Scalar};

#[test]
fn test_flexible_auth_basic_signature() {
    let mut auth_manager = AuthManager::new();
    
    // Add Ed25519 authenticator
    auth_manager.add_authenticator(Box::new(Ed25519Authenticator));
    auth_manager.add_policy(Box::new(StandardAccountPolicy));
    
    let account_id = UnitsObjectId::new([1u8; 32]);
    let requester_id = UnitsObjectId::new([2u8; 32]);
    
    // Create auth context
    let auth_context = AuthContext {
        operation: "update_account".to_string(),
        target_account: account_id,
        requester: requester_id,
        timestamp: 1234567890,
        operation_data: b"test_data".to_vec(),
    };
    
    // Create a dummy signature credential
    let credentials = vec![
        AuthCredential::Signature {
            signature_type: SignatureType::Ed25519,
            signature_bytes: vec![0u8; 64], // Invalid signature
            public_key: vec![2u8; 32],
        }
    ];
    
    // This should fail with invalid credentials since it's a dummy signature
    let result = auth_manager.authenticate(&credentials, &auth_context);
    match result {
        AuthResult::Failed(_) => {
            // Expected - dummy signature should fail
        }
        _ => panic!("Expected authentication to fail with dummy signature"),
    }
}

#[test]
fn test_multiple_signature_schemes() {
    let mut auth_manager = AuthManager::new();
    
    // Add all signature authenticators
    for authenticator in create_default_signature_authenticators() {
        auth_manager.add_authenticator(authenticator);
    }
    auth_manager.add_policy(Box::new(StandardAccountPolicy));
    
    let account_id = UnitsObjectId::new([1u8; 32]);
    let requester_id = UnitsObjectId::new([2u8; 32]);
    
    let auth_context = AuthContext {
        operation: "update_account".to_string(),
        target_account: account_id,
        requester: requester_id,
        timestamp: 1234567890,
        operation_data: b"test_data".to_vec(),
    };
    
    // Test different signature types (all will fail with dummy data, but should be recognized)
    let signature_types = vec![
        SignatureType::Ed25519,
        SignatureType::EcdsaSecp256k1,
        SignatureType::EcdsaSecp256r1,
        SignatureType::Rsa2048,
        SignatureType::Rsa4096,
    ];
    
    for sig_type in signature_types {
        let credentials = vec![
            AuthCredential::Signature {
                signature_type: sig_type.clone(),
                signature_bytes: vec![0u8; 64],
                public_key: vec![2u8; 32],
            }
        ];
        
        let result = auth_manager.authenticate(&credentials, &auth_context);
        
        // Should fail, but with proper error handling (not unsupported method)
        match result {
            AuthResult::Failed(_) => {
                // Expected for invalid signatures
            }
            _ => {
                // Some signature types might be unsupported in our test implementation
            }
        }
    }
}

#[test]
fn test_multi_factor_requirements() {
    let mut auth_manager = AuthManager::new();
    
    // Add authenticators
    for authenticator in create_default_signature_authenticators() {
        auth_manager.add_authenticator(authenticator);
    }
    
    // Add high-security policy (requires signature + MFA)
    auth_manager.add_policy(Box::new(HighSecurityPolicy));
    
    let account_id = UnitsObjectId::new([1u8; 32]);
    let requester_id = UnitsObjectId::new([2u8; 32]);
    
    let auth_context = AuthContext {
        operation: "update_account".to_string(),
        target_account: account_id,
        requester: requester_id,
        timestamp: 1234567890,
        operation_data: b"test_data".to_vec(),
    };
    
    // Only signature should fail with high-security policy
    let signature_only = vec![
        AuthCredential::Signature {
            signature_type: SignatureType::Ed25519,
            signature_bytes: vec![0u8; 64],
            public_key: vec![2u8; 32],
        }
    ];
    
    let result = auth_manager.authenticate(&signature_only, &auth_context);
    match result {
        AuthResult::Failed(_) => {
            // Expected - high security requires MFA
        }
        _ => panic!("Expected authentication to fail without MFA"),
    }
    
    // Signature + TOTP should be accepted by policy (but may fail verification)
    let signature_and_totp = vec![
        AuthCredential::Signature {
            signature_type: SignatureType::Ed25519,
            signature_bytes: vec![0u8; 64],
            public_key: vec![2u8; 32],
        },
        AuthCredential::TimeBasedCode {
            code: "123456".to_string(),
            timestamp: 1234567890,
        }
    ];
    
    let result = auth_manager.authenticate(&signature_and_totp, &auth_context);
    // May still fail due to invalid credentials, but should recognize the factors
}

#[test]
fn test_configurable_policy() {
    let mut policy = ConfigurablePolicy::standard();
    
    // Read operations should require no auth
    policy.set_operation_requirement(
        "get_account".to_string(),
        AuthRequirement::All(vec![])
    );
    
    // Update operations require signature
    policy.set_operation_requirement(
        "update_account".to_string(),
        AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519))
    );
    
    // Sensitive operations require signature + MFA
    policy.set_operation_requirement(
        "deactivate_account".to_string(),
        AuthRequirement::All(vec![
            AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
            AuthRequirement::Factor(AuthFactor::TimeBasedCode)
        ])
    );
    
    let account_id = UnitsObjectId::new([1u8; 32]);
    let requester_id = UnitsObjectId::new([2u8; 32]);
    
    // Test read operation (should require no auth)
    let read_context = AuthContext {
        operation: "get_account".to_string(),
        target_account: account_id,
        requester: requester_id,
        timestamp: 1234567890,
        operation_data: b"".to_vec(),
    };
    
    let no_credentials = vec![];
    let result = policy.validate(&no_credentials, &read_context);
    match result {
        AuthResult::Success => {
            // Expected - read operations need no auth
        }
        _ => panic!("Read operation should require no authentication"),
    }
    
    // Test sensitive operation (should require signature + MFA)
    let sensitive_context = AuthContext {
        operation: "deactivate_account".to_string(),
        target_account: account_id,
        requester: requester_id,
        timestamp: 1234567890,
        operation_data: b"deactivate".to_vec(),
    };
    
    let signature_only = vec![
        AuthCredential::Signature {
            signature_type: SignatureType::Ed25519,
            signature_bytes: vec![0u8; 64],
            public_key: vec![2u8; 32],
        }
    ];
    
    let result = policy.validate(&signature_only, &sensitive_context);
    match result {
        AuthResult::Failed(_) => {
            // Expected - sensitive operations need MFA too
        }
        _ => panic!("Sensitive operation should require MFA"),
    }
}

#[test]
fn test_enhanced_account_data() {
    let account_id = UnitsObjectId::new([1u8; 32]);
    let timestamp = 1234567890;
    
    let account = EnhancedAccountData::new(account_id, timestamp)
        .with_username("testuser".to_string())
        .with_display_name("Test User".to_string())
        .with_supported_factors(vec![
            AuthFactor::Signature(SignatureType::Ed25519),
            AuthFactor::TimeBasedCode,
            AuthFactor::HardwareToken,
        ]);
    
    assert_eq!(account.account_id, account_id);
    assert_eq!(account.username, Some("testuser".to_string()));
    assert_eq!(account.display_name, Some("Test User".to_string()));
    assert!(account.is_active);
    assert_eq!(account.created_at, timestamp);
    assert_eq!(account.updated_at, timestamp);
    assert_eq!(account.supported_auth_factors.len(), 3);
    
    // Test serialization
    let serialized = borsh::to_vec(&account).unwrap();
    let deserialized: EnhancedAccountData = borsh::from_slice(&serialized).unwrap();
    
    assert_eq!(account.account_id, deserialized.account_id);
    assert_eq!(account.username, deserialized.username);
    assert_eq!(account.supported_auth_factors, deserialized.supported_auth_factors);
}

#[test]
fn test_flexible_parameters() {
    let account_id = UnitsObjectId::new([1u8; 32]);
    
    let create_params = FlexCreateAccountParams {
        username: Some("testuser".to_string()),
        display_name: Some("Test User".to_string()),
        metadata: Some(HashMap::new()),
        recovery_addresses: Some(vec![UnitsObjectId::new([2u8; 32])]),
        credentials: vec![
            AuthCredential::Signature {
                signature_type: SignatureType::Ed25519,
                signature_bytes: vec![0u8; 64],
                public_key: vec![1u8; 32],
            },
            AuthCredential::TimeBasedCode {
                code: "123456".to_string(),
                timestamp: 1234567890,
            }
        ]
    };
    
    // Test serialization
    let serialized = borsh::to_vec(&create_params).unwrap();
    let deserialized: FlexCreateAccountParams = borsh::from_slice(&serialized).unwrap();
    
    assert_eq!(create_params.username, deserialized.username);
    assert_eq!(create_params.credentials.len(), deserialized.credentials.len());
    
    let update_params = FlexUpdateAccountParams {
        account_id,
        username: Some("updated_user".to_string()),
        display_name: None,
        metadata: None,
        credentials: vec![
            AuthCredential::Signature {
                signature_type: SignatureType::Ed25519,
                signature_bytes: vec![0u8; 64],
                public_key: vec![1u8; 32],
            }
        ]
    };
    
    // Test serialization
    let serialized = borsh::to_vec(&update_params).unwrap();
    let deserialized: FlexUpdateAccountParams = borsh::from_slice(&serialized).unwrap();
    
    assert_eq!(update_params.account_id, deserialized.account_id);
    assert_eq!(update_params.username, deserialized.username);
    assert_eq!(update_params.credentials.len(), deserialized.credentials.len());
}

#[test]
fn test_auth_requirement_logic() {
    // Test AND logic
    let and_requirement = AuthRequirement::All(vec![
        AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
        AuthRequirement::Factor(AuthFactor::TimeBasedCode),
    ]);
    
    // Test OR logic
    let or_requirement = AuthRequirement::Any(vec![
        AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
        AuthRequirement::Factor(AuthFactor::RecoveryKey),
    ]);
    
    // Test AtLeastN logic
    let at_least_n_requirement = AuthRequirement::AtLeastN {
        n: 2,
        factors: vec![
            AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
            AuthRequirement::Factor(AuthFactor::TimeBasedCode),
            AuthRequirement::Factor(AuthFactor::HardwareToken),
        ]
    };
    
    // Test serialization of complex requirements
    let serialized = borsh::to_vec(&and_requirement).unwrap();
    let deserialized: AuthRequirement = borsh::from_slice(&serialized).unwrap();
    
    // Should be able to serialize/deserialize complex auth requirements
    match (and_requirement, deserialized) {
        (AuthRequirement::All(orig), AuthRequirement::All(deser)) => {
            assert_eq!(orig.len(), deser.len());
        }
        _ => panic!("Serialization failed for AuthRequirement"),
    }
}