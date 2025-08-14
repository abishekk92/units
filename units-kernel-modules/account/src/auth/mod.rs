#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{vec::Vec, string::String, boxed::Box};

use units_kernel_sdk::UnitsObjectId;
use borsh::{BorshDeserialize, BorshSerialize};

pub mod signature_schemes;
pub mod policies;
pub mod multi_factor;

/// Authentication result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    Success,
    Failed(AuthError),
    Pending(PendingAuth),
}

/// Authentication errors
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum AuthError {
    InvalidCredentials,
    InsufficientAuth,
    UnsupportedMethod,
    ExpiredCredentials,
    RateLimited,
    MultiFactorRequired,
    InvalidPolicy,
}

/// Pending authentication (for multi-step auth)
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct PendingAuth {
    pub session_id: String,
    pub required_factors: Vec<AuthFactor>,
    pub completed_factors: Vec<AuthFactor>,
}

/// Authentication factor types
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum AuthFactor {
    Signature(SignatureType),
    TimeBasedCode,
    HardwareToken,
    Biometric,
    RecoveryKey,
}

/// Supported signature types
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum SignatureType {
    Ed25519,
    EcdsaSecp256k1,
    EcdsaSecp256r1,
    Rsa2048,
    Rsa4096,
}

/// Authentication context for operations
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct AuthContext {
    pub operation: String,
    pub target_account: UnitsObjectId,
    pub requester: UnitsObjectId,
    pub timestamp: u64,
    pub operation_data: Vec<u8>,
}

/// Authentication credential
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum AuthCredential {
    Signature {
        signature_type: SignatureType,
        signature_bytes: Vec<u8>,
        public_key: Vec<u8>,
    },
    TimeBasedCode {
        code: String,
        timestamp: u64,
    },
    HardwareToken {
        token_id: String,
        challenge_response: Vec<u8>,
    },
    Biometric {
        biometric_type: String,
        template_hash: Vec<u8>,
    },
    RecoveryKey {
        recovery_address: UnitsObjectId,
        signature: Vec<u8>,
    },
}

/// Main authentication trait
pub trait Authenticator {
    /// Verify authentication credential against context
    fn verify(&self, credential: &AuthCredential, context: &AuthContext) -> AuthResult;
    
    /// Get supported authentication factors
    fn supported_factors(&self) -> Vec<AuthFactor>;
    
    /// Check if this authenticator can handle the credential
    fn can_handle(&self, credential: &AuthCredential) -> bool;
}

/// Authorization policy trait
pub trait AuthPolicy {
    /// Determine required authentication factors for an operation
    fn required_auth(&self, context: &AuthContext) -> Vec<AuthRequirement>;
    
    /// Validate if provided credentials satisfy the policy
    fn validate(&self, credentials: &[AuthCredential], context: &AuthContext) -> AuthResult;
}

/// Authentication requirement (supports AND/OR logic)
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum AuthRequirement {
    /// Single factor required
    Factor(AuthFactor),
    /// All factors required (AND)
    All(Vec<AuthRequirement>),
    /// Any factor acceptable (OR) 
    Any(Vec<AuthRequirement>),
    /// At least N factors required
    AtLeastN {
        n: usize,
        factors: Vec<AuthRequirement>,
    },
}

/// Main authentication manager
pub struct AuthManager {
    authenticators: Vec<Box<dyn Authenticator>>,
    policies: Vec<Box<dyn AuthPolicy>>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            authenticators: Vec::new(),
            policies: Vec::new(),
        }
    }
    
    /// Add an authenticator
    pub fn add_authenticator(&mut self, authenticator: Box<dyn Authenticator>) {
        self.authenticators.push(authenticator);
    }
    
    /// Add an authorization policy
    pub fn add_policy(&mut self, policy: Box<dyn AuthPolicy>) {
        self.policies.push(policy);
    }
    
    /// Authenticate operation with provided credentials
    pub fn authenticate(
        &self,
        credentials: &[AuthCredential],
        context: &AuthContext,
    ) -> AuthResult {
        // Get required authentication from policies
        let mut all_requirements = Vec::new();
        for policy in &self.policies {
            all_requirements.extend(policy.required_auth(context));
        }
        
        if all_requirements.is_empty() {
            return AuthResult::Success; // No authentication required
        }
        
        // Validate each credential
        let mut verified_factors = Vec::new();
        for credential in credentials {
            for authenticator in &self.authenticators {
                if authenticator.can_handle(credential) {
                    match authenticator.verify(credential, context) {
                        AuthResult::Success => {
                            if let Some(factor) = self.credential_to_factor(credential) {
                                verified_factors.push(factor);
                            }
                        }
                        AuthResult::Failed(err) => return AuthResult::Failed(err),
                        AuthResult::Pending(pending) => return AuthResult::Pending(pending),
                    }
                    break;
                }
            }
        }
        
        // Check if requirements are satisfied
        self.check_requirements(&all_requirements, &verified_factors)
    }
    
    fn credential_to_factor(&self, credential: &AuthCredential) -> Option<AuthFactor> {
        match credential {
            AuthCredential::Signature { signature_type, .. } => {
                Some(AuthFactor::Signature(signature_type.clone()))
            }
            AuthCredential::TimeBasedCode { .. } => Some(AuthFactor::TimeBasedCode),
            AuthCredential::HardwareToken { .. } => Some(AuthFactor::HardwareToken),
            AuthCredential::Biometric { .. } => Some(AuthFactor::Biometric),
            AuthCredential::RecoveryKey { .. } => Some(AuthFactor::RecoveryKey),
        }
    }
    
    fn check_requirements(
        &self,
        requirements: &[AuthRequirement],
        verified_factors: &[AuthFactor],
    ) -> AuthResult {
        for requirement in requirements {
            if !self.check_single_requirement(requirement, verified_factors) {
                return AuthResult::Failed(AuthError::InsufficientAuth);
            }
        }
        AuthResult::Success
    }
    
    fn check_single_requirement(
        &self,
        requirement: &AuthRequirement,
        verified_factors: &[AuthFactor],
    ) -> bool {
        match requirement {
            AuthRequirement::Factor(factor) => verified_factors.contains(factor),
            AuthRequirement::All(sub_requirements) => {
                sub_requirements.iter().all(|req| self.check_single_requirement(req, verified_factors))
            }
            AuthRequirement::Any(sub_requirements) => {
                sub_requirements.iter().any(|req| self.check_single_requirement(req, verified_factors))
            }
            AuthRequirement::AtLeastN { n, factors } => {
                let satisfied = factors.iter()
                    .filter(|req| self.check_single_requirement(req, verified_factors))
                    .count();
                satisfied >= *n
            }
        }
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}