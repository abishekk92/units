# Cryptographic Signature Verification for Account Module

## Overview

The UNITS account module now includes comprehensive Ed25519 signature verification to ensure that only the rightful owner of an account can perform operations. This enhances security by requiring cryptographic proof of authorization for all account modifications.

## Features Added

### 1. **Ed25519 Cryptographic Primitives**
- **Location**: `src/crypto.rs`
- **Ed25519 signature verification** using `curve25519-dalek`
- **Public key operations** for converting between `UnitsObjectId` and Ed25519 public keys
- **Message creation** for deterministic signing

### 2. **Signature-Protected Operations**

All sensitive account operations now require valid signatures:

- ✅ **Account Updates** (`update_account`) - Required
- ✅ **Add Recovery Address** (`add_recovery_address`) - Required  
- ✅ **Remove Recovery Address** (`remove_recovery_address`) - Required
- ✅ **Deactivate Account** (`deactivate_account`) - Required
- ✅ **Reactivate Account** (`reactivate_account`) - Required
- ⚪ **Create Account** (`create_account`) - Optional (for new accounts)
- ⚪ **Get Account** (`get_account`) - Read-only, no signature needed

### 3. **Security Model**

#### **Signature Verification Process**:
1. **Extract public key** from the `UnitsObjectId` (account controller)
2. **Create deterministic message** containing:
   - Operation name (e.g., "update_account")
   - Account ID
   - Timestamp
   - Operation parameters (excluding signature)
3. **Verify Ed25519 signature** against the message using the public key
4. **Authorize operation** only if signature is valid

#### **Message Format**:
```
Message = operation_name || account_id || timestamp || params
```

#### **Authorization Hierarchy**:
- **Primary Controller**: Account's `controller_id` can perform all operations
- **Recovery Addresses**: Can reactivate deactivated accounts
- **Signature Required**: All mutations must be cryptographically signed

### 4. **Data Structures**

#### **Enhanced Parameter Structures**:
```rust
pub struct UpdateAccountParams {
    pub account_id: UnitsObjectId,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub signature: Signature, // Required for account updates
}
```

#### **Signature Structure**:
```rust
pub struct Signature {
    pub bytes: [u8; 64], // Ed25519 signature (R + S)
}
```

#### **Public Key Operations**:
```rust
pub struct PublicKey {
    pub point: EdwardsPoint,
    pub compressed: CompressedEdwardsY,
}
```

### 5. **Error Handling**

New error codes for signature verification:
- `ERROR_SIGNATURE_VERIFICATION_FAILED` (1010)
- `ERROR_INVALID_SIGNATURE` (1011)  
- `ERROR_MISSING_SIGNATURE` (1012)

### 6. **Testing Coverage**

Comprehensive tests verify:
- ✅ **Signature creation and serialization**
- ✅ **Public key conversions** (UnitsObjectId ↔ Ed25519 public key)
- ✅ **Message creation** for deterministic signing
- ✅ **Account data serialization** with new signature fields
- ✅ **Username validation** (unchanged)

## Usage Example

To perform an account update with signature verification:

```rust
use account::{UpdateAccountParams, crypto::Signature};
use units_kernel_sdk::UnitsObjectId;

// Create update parameters with signature
let params = UpdateAccountParams {
    account_id: my_account_id,
    username: Some("new_username".to_string()),
    display_name: Some("New Display Name".to_string()),
    metadata: Some(metadata_map),
    signature: my_ed25519_signature, // Required!
};

// The kernel module will:
// 1. Verify the signature against the account controller's public key
// 2. Only proceed if the signature is valid
// 3. Return Unauthorized error if signature verification fails
```

## Security Benefits

1. **Cryptographic Proof**: Ed25519 signatures provide 128-bit security
2. **Non-repudiation**: Signatures prove the account owner authorized the operation
3. **Replay Protection**: Timestamps in messages prevent replay attacks
4. **Controller Verification**: Only the holder of the private key can generate valid signatures
5. **Recovery Mechanism**: Recovery addresses can still reactivate accounts if primary key is lost

## Integration with UNITS Architecture

- **Compatible** with existing proof-based verification system
- **Extends** the controller-based authorization model
- **Maintains** all existing account management features
- **Follows** established kernel module patterns
- **RISC-V compatible** for VM execution

The signature verification system provides a robust foundation for secure account management while maintaining compatibility with the broader UNITS ecosystem.