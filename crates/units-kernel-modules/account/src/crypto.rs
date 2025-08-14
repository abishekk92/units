#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;

use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;
use sha2::{Sha512, Digest};
use units_kernel_sdk::UnitsObjectId;
use borsh::{BorshDeserialize, BorshSerialize};

/// Ed25519 signature size in bytes
pub const SIGNATURE_SIZE: usize = 64;

/// Ed25519 public key size in bytes
pub const PUBLIC_KEY_SIZE: usize = 32;

/// Ed25519 signature
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Signature {
    pub bytes: [u8; SIGNATURE_SIZE],
}

impl Signature {
    pub fn new(bytes: [u8; SIGNATURE_SIZE]) -> Self {
        Self { bytes }
    }
    
    pub fn from_slice(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != SIGNATURE_SIZE {
            return Err(CryptoError::InvalidSignatureLength);
        }
        let mut sig_bytes = [0u8; SIGNATURE_SIZE];
        sig_bytes.copy_from_slice(bytes);
        Ok(Self::new(sig_bytes))
    }
    
    pub fn to_bytes(&self) -> [u8; SIGNATURE_SIZE] {
        self.bytes
    }
}

/// Ed25519 public key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey {
    pub point: EdwardsPoint,
    pub compressed: CompressedEdwardsY,
}

impl PublicKey {
    pub fn from_bytes(bytes: &[u8; PUBLIC_KEY_SIZE]) -> Result<Self, CryptoError> {
        let compressed = CompressedEdwardsY(*bytes);
        let point = compressed.decompress()
            .ok_or(CryptoError::InvalidPublicKey)?;
        
        Ok(Self {
            point,
            compressed,
        })
    }
    
    pub fn from_units_object_id(id: &UnitsObjectId) -> Result<Self, CryptoError> {
        if id.bytes().len() != PUBLIC_KEY_SIZE {
            return Err(CryptoError::InvalidPublicKey);
        }
        
        let mut bytes = [0u8; PUBLIC_KEY_SIZE];
        bytes.copy_from_slice(id.bytes());
        Self::from_bytes(&bytes)
    }
    
    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_SIZE] {
        self.compressed.to_bytes()
    }
    
    pub fn to_units_object_id(&self) -> UnitsObjectId {
        UnitsObjectId::new(self.to_bytes())
    }
}

/// Cryptographic errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    InvalidSignatureLength,
    InvalidPublicKey,
    InvalidSignature,
    SignatureVerificationFailed,
}

/// Verify an Ed25519 signature
pub fn verify_signature(
    public_key: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> Result<(), CryptoError> {
    // Extract R and S from signature
    let r_bytes = &signature.bytes[..32];
    let s_bytes = &signature.bytes[32..];
    
    // Parse R point
    let mut r_array = [0u8; 32];
    r_array.copy_from_slice(r_bytes);
    let r_compressed = CompressedEdwardsY(r_array);
    let r_point = r_compressed.decompress()
        .ok_or(CryptoError::InvalidSignature)?;
    
    // Parse S scalar
    let s_option = Scalar::from_canonical_bytes(*array_ref!(s_bytes, 0, 32));
    let s_scalar = if s_option.is_some().unwrap_u8() == 1 {
        s_option.unwrap()
    } else {
        return Err(CryptoError::InvalidSignature);
    };
    
    // Compute hash H(R || A || M)
    let mut hasher = Sha512::new();
    hasher.update(r_bytes);
    hasher.update(&public_key.to_bytes());
    hasher.update(message);
    let hash = hasher.finalize();
    
    // Convert hash to scalar
    let h_scalar = Scalar::from_bytes_mod_order_wide(array_ref!(hash, 0, 64));
    
    // Verify: [S]B = R + [H(R||A||M)]A
    let left_side = s_scalar * constants::ED25519_BASEPOINT_POINT;
    let right_side = r_point + (h_scalar * public_key.point);
    
    if left_side == right_side {
        Ok(())
    } else {
        Err(CryptoError::SignatureVerificationFailed)
    }
}

/// Create a message digest for signing account operations
pub fn create_operation_message(
    operation: &str,
    account_id: &UnitsObjectId,
    timestamp: u64,
    params: &[u8],
) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(operation.as_bytes());
    message.extend_from_slice(account_id.bytes());
    message.extend_from_slice(&timestamp.to_le_bytes());
    message.extend_from_slice(params);
    message
}

// Use arrayref crate
use arrayref::array_ref;

// Re-export Ed25519 constants
use curve25519_dalek::constants;

#[cfg(test)]
mod tests {
    use super::*;
    use curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
    use curve25519_dalek::scalar::Scalar;
    use alloc::vec;
    
    #[test]
    fn test_public_key_conversion() {
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
    fn test_signature_creation() {
        let sig_bytes = [1u8; SIGNATURE_SIZE];
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
}