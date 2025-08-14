# Flexible Authentication Architecture for Account Module

## Overview

The UNITS account module now supports a comprehensive, flexible authentication system that can handle multiple signature schemes, multi-factor authentication, and configurable authorization policies. This system is designed to be extensible and adaptable to various security requirements.

## Architecture Components

### 1. Authentication Framework (`auth/mod.rs`)

**Core Traits:**
- `Authenticator`: Verifies authentication credentials against contexts
- `AuthPolicy`: Determines required authentication factors for operations

**Key Types:**
- `AuthCredential`: Represents different types of credentials (signatures, TOTP codes, hardware tokens, etc.)
- `AuthRequirement`: Supports complex authentication logic (AND, OR, AtLeastN)
- `AuthManager`: Orchestrates authentication with multiple authenticators and policies

### 2. Signature Schemes (`auth/signature_schemes.rs`)

**Supported Signature Types:**
- ✅ **Ed25519**: Fully implemented (existing crypto module)
- 🔧 **ECDSA secp256k1**: Interface ready (requires secp256k1 crate)
- 🔧 **ECDSA secp256r1**: Interface ready (requires p256 crate)
- 🔧 **RSA 2048/4096**: Interface ready (requires rsa crate)
- ✅ **Recovery Keys**: Uses Ed25519 with recovery addresses

**Implementation Status:**
```rust
// Currently implemented
Ed25519Authenticator                    // ✅ Working
RecoveryKeyAuthenticator                // ✅ Working

// Ready for implementation (need crate dependencies)
EcdsaSecp256k1Authenticator            // 🔧 Interface ready
EcdsaSecp256r1Authenticator            // 🔧 Interface ready  
RsaAuthenticator                       // 🔧 Interface ready
```

### 3. Multi-Factor Authentication (`auth/multi_factor.rs`)

**Supported MFA Types:**
- 🔧 **TOTP (Time-based OTP)**: Basic implementation (needs HMAC library)
- 🔧 **Hardware Tokens**: Interface for FIDO2/WebAuthn style tokens
- 🔧 **Biometric**: Template-based biometric authentication

**Features:**
- TOTP with configurable periods, digits, and hash algorithms
- Hardware token registration and challenge-response
- Biometric template enrollment and verification

### 4. Authorization Policies (`auth/policies.rs`)

**Policy Types:**

**StandardAccountPolicy:**
- Read operations: No authentication
- Account creation: No authentication (new accounts)
- Standard operations: Ed25519 signature required
- Reactivation: Owner signature OR recovery key

**HighSecurityPolicy:**
- Read operations: No authentication
- Standard operations: Signature + MFA required
- Sensitive operations: Signature + TOTP required
- Reactivation: (Signature + MFA) OR (≥2 recovery keys)

**ConfigurablePolicy:**
- Fully customizable per operation
- Supports complex requirement logic
- Serializable for storage

### 5. Enhanced Module (`enhanced_module.rs`)

**New Functions:**
- `flex_create_account`: Create account with flexible auth
- `flex_update_account`: Update account with flexible auth
- `flex_add_recovery_address`: Add recovery with flexible auth
- `flex_remove_recovery_address`: Remove recovery with flexible auth
- `flex_deactivate_account`: Deactivate with flexible auth
- `flex_reactivate_account`: Reactivate with flexible auth

## Authentication Flow

```
1. Operation Request
   └── Contains: operation, credentials[], target_account
   
2. Policy Evaluation
   └── Determines required AuthRequirement[]
   
3. Credential Verification
   └── Each credential verified by appropriate Authenticator
   
4. Requirement Satisfaction
   └── Check if verified credentials satisfy requirements
   
5. Authorization Decision
   └── Success/Failed/Pending
```

## Usage Examples

### Basic Ed25519 Authentication

```rust
use account::auth::{AuthCredential, SignatureType};

let credentials = vec![
    AuthCredential::Signature {
        signature_type: SignatureType::Ed25519,
        signature_bytes: my_signature_bytes,
        public_key: my_public_key_bytes,
    }
];

// Use in FlexUpdateAccountParams
let params = FlexUpdateAccountParams {
    account_id: my_account,
    username: Some("newname".to_string()),
    credentials,
    ..
};
```

### Multi-Factor Authentication

```rust
let credentials = vec![
    // Primary signature
    AuthCredential::Signature {
        signature_type: SignatureType::Ed25519,
        signature_bytes: my_signature_bytes,
        public_key: my_public_key_bytes,
    },
    // TOTP code
    AuthCredential::TimeBasedCode {
        code: "123456".to_string(),
        timestamp: current_timestamp,
    }
];
```

### Recovery Using Multiple Keys

```rust
let credentials = vec![
    AuthCredential::RecoveryKey {
        recovery_address: recovery_address_1,
        signature: recovery_signature_1,
    },
    AuthCredential::RecoveryKey {
        recovery_address: recovery_address_2, 
        signature: recovery_signature_2,
    }
];
```

### Custom Authorization Policy

```rust
use account::auth::{AuthRequirement, AuthFactor, SignatureType};

let mut policy = ConfigurablePolicy::new();

// Require signature + hardware token for sensitive operations
policy.set_operation_requirement(
    "deactivate_account".to_string(),
    AuthRequirement::All(vec![
        AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
        AuthRequirement::Factor(AuthFactor::HardwareToken),
    ])
);

// Allow multiple recovery methods
policy.set_operation_requirement(
    "reactivate_account".to_string(),
    AuthRequirement::Any(vec![
        // Owner with MFA
        AuthRequirement::All(vec![
            AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
            AuthRequirement::Factor(AuthFactor::TimeBasedCode),
        ]),
        // Multiple recovery signatures
        AuthRequirement::AtLeastN {
            n: 2,
            factors: vec![AuthRequirement::Factor(AuthFactor::RecoveryKey)],
        }
    ])
);
```

## Enhanced Account Data

```rust
use account::{EnhancedAccountData, auth::AuthFactor};

let account = EnhancedAccountData::new(account_id, timestamp)
    .with_username("user123".to_string())
    .with_supported_factors(vec![
        AuthFactor::Signature(SignatureType::Ed25519),
        AuthFactor::TimeBasedCode,
        AuthFactor::HardwareToken,
    ])
    .with_auth_policy(serialized_custom_policy);
```

## Module Configuration

### Standard Security
```rust
let module = EnhancedAccountModule::new_standard();
// - Ed25519 signatures
// - Recovery keys
// - Standard policies
```

### High Security
```rust
let module = EnhancedAccountModule::new_high_security();
// - All signature schemes
// - MFA required
// - Strict policies
```

### Custom Configuration
```rust
let mut auth_manager = AuthManager::new();

// Add specific authenticators
auth_manager.add_authenticator(Box::new(Ed25519Authenticator));
auth_manager.add_authenticator(Box::new(TotpAuthenticator::new()));

// Add custom policies
auth_manager.add_policy(Box::new(my_custom_policy));

let module = EnhancedAccountModule::new_custom(auth_manager);
```

## Security Benefits

1. **Multi-Signature Support**: Different signature algorithms for different security models
2. **Multi-Factor Authentication**: TOTP, hardware tokens, biometrics
3. **Flexible Policies**: Configurable authentication requirements per operation
4. **Recovery Mechanisms**: Multiple recovery methods with flexible requirements
5. **Extensibility**: Easy to add new authentication methods
6. **Backward Compatibility**: Legacy single-signature operations still supported

## Implementation Status

✅ **Complete:**
- Core authentication framework
- Ed25519 signature verification
- Flexible parameter structures
- Enhanced account data
- Policy framework
- Basic TOTP interface
- Comprehensive test suite

🔧 **Ready for Implementation:**
- Additional signature schemes (secp256k1, secp256r1, RSA)
- Full TOTP implementation with HMAC
- Hardware token integration
- Biometric authentication
- Policy persistence in storage

## Migration Path

### Phase 1: Backward Compatibility
- Keep existing `AccountModule` for legacy support
- Introduce `EnhancedAccountModule` alongside

### Phase 2: Feature Adoption
- Applications can use `flex_*` functions for new features
- Gradual migration of authentication requirements

### Phase 3: Full Migration
- Deprecate legacy functions
- All operations use flexible authentication
- Enhanced account data becomes default

This architecture provides a solid foundation for evolving authentication requirements while maintaining compatibility with existing systems.