#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;

use super::{Authenticator, AuthCredential, AuthContext, AuthResult, AuthError, AuthFactor, SignatureType};
use crate::crypto;

/// Ed25519 signature authenticator
pub struct Ed25519Authenticator;

impl Authenticator for Ed25519Authenticator {
    fn verify(&self, credential: &AuthCredential, context: &AuthContext) -> AuthResult {
        if let AuthCredential::Signature { signature_type, signature_bytes, public_key } = credential {
            if *signature_type != SignatureType::Ed25519 {
                return AuthResult::Failed(AuthError::UnsupportedMethod);
            }
            
            // Convert to our crypto types
            let signature = match crypto::Signature::from_slice(signature_bytes) {
                Ok(sig) => sig,
                Err(_) => return AuthResult::Failed(AuthError::InvalidCredentials),
            };
            
            let public_key = match crypto::PublicKey::from_bytes(
                &public_key.clone().try_into().map_err(|_| AuthError::InvalidCredentials).unwrap()
            ) {
                Ok(pk) => pk,
                Err(_) => return AuthResult::Failed(AuthError::InvalidCredentials),
            };
            
            // Create message for verification
            let message = crypto::create_operation_message(
                &context.operation,
                &context.target_account,
                context.timestamp,
                &context.operation_data,
            );
            
            // Verify signature
            match crypto::verify_signature(&public_key, &message, &signature) {
                Ok(_) => {
                    // Check if public key matches the requester
                    if public_key.to_units_object_id() == context.requester {
                        AuthResult::Success
                    } else {
                        AuthResult::Failed(AuthError::InvalidCredentials)
                    }
                }
                Err(_) => AuthResult::Failed(AuthError::InvalidCredentials),
            }
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        vec![AuthFactor::Signature(SignatureType::Ed25519)]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        matches!(
            credential,
            AuthCredential::Signature { 
                signature_type: SignatureType::Ed25519, 
                .. 
            }
        )
    }
}

/// ECDSA secp256k1 signature authenticator (Bitcoin/Ethereum style)
pub struct EcdsaSecp256k1Authenticator;

impl Authenticator for EcdsaSecp256k1Authenticator {
    fn verify(&self, credential: &AuthCredential, _context: &AuthContext) -> AuthResult {
        if let AuthCredential::Signature { signature_type, .. } = credential {
            if *signature_type != SignatureType::EcdsaSecp256k1 {
                return AuthResult::Failed(AuthError::UnsupportedMethod);
            }
            
            // TODO: Implement secp256k1 verification
            // This would require adding secp256k1 crate dependency
            AuthResult::Failed(AuthError::UnsupportedMethod)
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        vec![AuthFactor::Signature(SignatureType::EcdsaSecp256k1)]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        matches!(
            credential,
            AuthCredential::Signature { 
                signature_type: SignatureType::EcdsaSecp256k1, 
                .. 
            }
        )
    }
}

/// ECDSA secp256r1 signature authenticator (NIST P-256)
pub struct EcdsaSecp256r1Authenticator;

impl Authenticator for EcdsaSecp256r1Authenticator {
    fn verify(&self, credential: &AuthCredential, _context: &AuthContext) -> AuthResult {
        if let AuthCredential::Signature { signature_type, .. } = credential {
            if *signature_type != SignatureType::EcdsaSecp256r1 {
                return AuthResult::Failed(AuthError::UnsupportedMethod);
            }
            
            // TODO: Implement secp256r1 verification
            // This would require adding p256 crate dependency
            AuthResult::Failed(AuthError::UnsupportedMethod)
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        vec![AuthFactor::Signature(SignatureType::EcdsaSecp256r1)]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        matches!(
            credential,
            AuthCredential::Signature { 
                signature_type: SignatureType::EcdsaSecp256r1, 
                .. 
            }
        )
    }
}

/// RSA signature authenticator
pub struct RsaAuthenticator {
    key_size: u32, // 2048 or 4096
}

impl RsaAuthenticator {
    pub fn new(key_size: u32) -> Self {
        Self { key_size }
    }
}

impl Authenticator for RsaAuthenticator {
    fn verify(&self, credential: &AuthCredential, _context: &AuthContext) -> AuthResult {
        if let AuthCredential::Signature { signature_type, .. } = credential {
            let expected_type = match self.key_size {
                2048 => SignatureType::Rsa2048,
                4096 => SignatureType::Rsa4096,
                _ => return AuthResult::Failed(AuthError::UnsupportedMethod),
            };
            
            if *signature_type != expected_type {
                return AuthResult::Failed(AuthError::UnsupportedMethod);
            }
            
            // TODO: Implement RSA verification
            // This would require adding rsa crate dependency
            AuthResult::Failed(AuthError::UnsupportedMethod)
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        let signature_type = match self.key_size {
            2048 => SignatureType::Rsa2048,
            4096 => SignatureType::Rsa4096,
            _ => return vec![],
        };
        vec![AuthFactor::Signature(signature_type)]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        if let AuthCredential::Signature { signature_type, .. } = credential {
            let expected_type = match self.key_size {
                2048 => SignatureType::Rsa2048,
                4096 => SignatureType::Rsa4096,
                _ => return false,
            };
            *signature_type == expected_type
        } else {
            false
        }
    }
}

/// Recovery key authenticator (uses account recovery addresses)
pub struct RecoveryKeyAuthenticator;

impl Authenticator for RecoveryKeyAuthenticator {
    fn verify(&self, credential: &AuthCredential, context: &AuthContext) -> AuthResult {
        if let AuthCredential::RecoveryKey { recovery_address, signature } = credential {
            // Create Ed25519 signature from bytes
            let ed25519_signature = match crypto::Signature::from_slice(signature) {
                Ok(sig) => sig,
                Err(_) => return AuthResult::Failed(AuthError::InvalidCredentials),
            };
            
            // Get public key from recovery address
            let public_key = match crypto::PublicKey::from_units_object_id(recovery_address) {
                Ok(pk) => pk,
                Err(_) => return AuthResult::Failed(AuthError::InvalidCredentials),
            };
            
            // Create message for verification  
            let message = crypto::create_operation_message(
                &context.operation,
                &context.target_account,
                context.timestamp,
                &context.operation_data,
            );
            
            // Verify signature
            match crypto::verify_signature(&public_key, &message, &ed25519_signature) {
                Ok(_) => AuthResult::Success,
                Err(_) => AuthResult::Failed(AuthError::InvalidCredentials),
            }
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        vec![AuthFactor::RecoveryKey]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        matches!(credential, AuthCredential::RecoveryKey { .. })
    }
}

/// Helper function to create all default signature authenticators
pub fn create_default_signature_authenticators() -> Vec<Box<dyn Authenticator>> {
    vec![
        Box::new(Ed25519Authenticator),
        Box::new(EcdsaSecp256k1Authenticator),
        Box::new(EcdsaSecp256r1Authenticator),
        Box::new(RsaAuthenticator::new(2048)),
        Box::new(RsaAuthenticator::new(4096)),
        Box::new(RecoveryKeyAuthenticator),
    ]
}