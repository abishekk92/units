#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{vec::Vec, string::String, collections::BTreeMap};

use super::{
    AuthPolicy, AuthContext, AuthCredential, AuthResult, AuthError, 
    AuthRequirement, AuthFactor, SignatureType
};
use borsh::{BorshDeserialize, BorshSerialize};

/// Standard account operations policy
pub struct StandardAccountPolicy;

impl AuthPolicy for StandardAccountPolicy {
    fn required_auth(&self, context: &AuthContext) -> Vec<AuthRequirement> {
        match context.operation.as_str() {
            // Read operations require no authentication
            "get_account" => vec![],
            
            // Account creation can be done without signature (new accounts)
            "create_account" => vec![],
            
            // Standard operations require owner signature
            "update_account" | "add_recovery_address" | "remove_recovery_address" => {
                vec![AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519))]
            }
            
            // Deactivation requires owner signature
            "deactivate_account" => {
                vec![AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519))]
            }
            
            // Reactivation can use owner signature OR recovery key
            "reactivate_account" => {
                vec![AuthRequirement::Any(vec![
                    AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
                    AuthRequirement::Factor(AuthFactor::RecoveryKey),
                ])]
            }
            
            _ => vec![AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519))],
        }
    }
    
    fn validate(&self, credentials: &[AuthCredential], context: &AuthContext) -> AuthResult {
        let requirements = self.required_auth(context);
        
        // If no requirements, allow
        if requirements.is_empty() {
            return AuthResult::Success;
        }
        
        // Check each requirement is satisfied
        for requirement in &requirements {
            if !self.check_requirement(requirement, credentials, context) {
                return AuthResult::Failed(AuthError::InsufficientAuth);
            }
        }
        
        AuthResult::Success
    }
}

impl StandardAccountPolicy {
    fn check_requirement(
        &self,
        requirement: &AuthRequirement,
        credentials: &[AuthCredential],
        context: &AuthContext,
    ) -> bool {
        match requirement {
            AuthRequirement::Factor(factor) => {
                self.has_factor(factor, credentials, context)
            }
            AuthRequirement::All(sub_requirements) => {
                sub_requirements.iter().all(|req| self.check_requirement(req, credentials, context))
            }
            AuthRequirement::Any(sub_requirements) => {
                sub_requirements.iter().any(|req| self.check_requirement(req, credentials, context))
            }
            AuthRequirement::AtLeastN { n, factors } => {
                let satisfied = factors.iter()
                    .filter(|req| self.check_requirement(req, credentials, context))
                    .count();
                satisfied >= *n
            }
        }
    }
    
    fn has_factor(&self, factor: &AuthFactor, credentials: &[AuthCredential], _context: &AuthContext) -> bool {
        match factor {
            AuthFactor::Signature(sig_type) => {
                credentials.iter().any(|cred| {
                    if let AuthCredential::Signature { signature_type, .. } = cred {
                        signature_type == sig_type
                    } else {
                        false
                    }
                })
            }
            AuthFactor::RecoveryKey => {
                credentials.iter().any(|cred| {
                    matches!(cred, AuthCredential::RecoveryKey { .. })
                })
            }
            AuthFactor::TimeBasedCode => {
                credentials.iter().any(|cred| {
                    matches!(cred, AuthCredential::TimeBasedCode { .. })
                })
            }
            AuthFactor::HardwareToken => {
                credentials.iter().any(|cred| {
                    matches!(cred, AuthCredential::HardwareToken { .. })
                })
            }
            AuthFactor::Biometric => {
                credentials.iter().any(|cred| {
                    matches!(cred, AuthCredential::Biometric { .. })
                })
            }
        }
    }
}

/// High-security policy for sensitive operations
pub struct HighSecurityPolicy;

impl AuthPolicy for HighSecurityPolicy {
    fn required_auth(&self, context: &AuthContext) -> Vec<AuthRequirement> {
        match context.operation.as_str() {
            // Read operations require no authentication
            "get_account" => vec![],
            
            // Account creation still allows no signature
            "create_account" => vec![],
            
            // Standard operations require signature + MFA
            "update_account" | "add_recovery_address" => {
                vec![AuthRequirement::All(vec![
                    AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
                    AuthRequirement::Any(vec![
                        AuthRequirement::Factor(AuthFactor::TimeBasedCode),
                        AuthRequirement::Factor(AuthFactor::HardwareToken),
                    ])
                ])]
            }
            
            // Removing recovery and deactivation require signature + MFA
            "remove_recovery_address" | "deactivate_account" => {
                vec![AuthRequirement::All(vec![
                    AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
                    AuthRequirement::Factor(AuthFactor::TimeBasedCode),
                ])]
            }
            
            // Reactivation requires multiple recovery signatures
            "reactivate_account" => {
                vec![AuthRequirement::Any(vec![
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
                ])]
            }
            
            _ => vec![AuthRequirement::All(vec![
                AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
                AuthRequirement::Factor(AuthFactor::TimeBasedCode),
            ])],
        }
    }
    
    fn validate(&self, credentials: &[AuthCredential], context: &AuthContext) -> AuthResult {
        let requirements = self.required_auth(context);
        
        // Use same validation logic as StandardAccountPolicy
        let standard_policy = StandardAccountPolicy;
        for requirement in &requirements {
            if !standard_policy.check_requirement(requirement, credentials, context) {
                return AuthResult::Failed(AuthError::InsufficientAuth);
            }
        }
        
        AuthResult::Success
    }
}

/// Configurable policy that can be customized per account
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ConfigurablePolicy {
    /// Map from operation name to authentication requirements
    pub operation_requirements: BTreeMap<String, AuthRequirement>,
    /// Default requirement for unlisted operations
    pub default_requirement: AuthRequirement,
}

impl ConfigurablePolicy {
    pub fn new() -> Self {
        Self {
            operation_requirements: BTreeMap::new(),
            default_requirement: AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
        }
    }
    
    /// Set authentication requirement for a specific operation
    pub fn set_operation_requirement(&mut self, operation: String, requirement: AuthRequirement) {
        self.operation_requirements.insert(operation, requirement);
    }
    
    /// Set default requirement for unlisted operations
    pub fn set_default_requirement(&mut self, requirement: AuthRequirement) {
        self.default_requirement = requirement;
    }
    
    /// Create a standard policy configuration
    pub fn standard() -> Self {
        let mut policy = Self::new();
        
        // Read operations require no auth
        policy.set_operation_requirement(
            "get_account".to_string(),
            AuthRequirement::All(vec![]) // No requirements
        );
        
        // Account creation requires no auth
        policy.set_operation_requirement(
            "create_account".to_string(),
            AuthRequirement::All(vec![])
        );
        
        // Reactivation allows owner OR recovery
        policy.set_operation_requirement(
            "reactivate_account".to_string(),
            AuthRequirement::Any(vec![
                AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
                AuthRequirement::Factor(AuthFactor::RecoveryKey),
            ])
        );
        
        policy
    }
    
    /// Create a high-security policy configuration
    pub fn high_security() -> Self {
        let mut policy = Self::new();
        
        // Read operations require no auth
        policy.set_operation_requirement(
            "get_account".to_string(),
            AuthRequirement::All(vec![])
        );
        
        // Account creation requires no auth
        policy.set_operation_requirement(
            "create_account".to_string(),
            AuthRequirement::All(vec![])
        );
        
        // Most operations require signature + MFA
        let mfa_requirement = AuthRequirement::All(vec![
            AuthRequirement::Factor(AuthFactor::Signature(SignatureType::Ed25519)),
            AuthRequirement::Any(vec![
                AuthRequirement::Factor(AuthFactor::TimeBasedCode),
                AuthRequirement::Factor(AuthFactor::HardwareToken),
            ])
        ]);
        
        for operation in ["update_account", "add_recovery_address", "remove_recovery_address", "deactivate_account"] {
            policy.set_operation_requirement(operation.to_string(), mfa_requirement.clone());
        }
        
        // Reactivation requires multiple recovery signatures or owner + MFA
        policy.set_operation_requirement(
            "reactivate_account".to_string(),
            AuthRequirement::Any(vec![
                mfa_requirement,
                AuthRequirement::AtLeastN {
                    n: 2,
                    factors: vec![AuthRequirement::Factor(AuthFactor::RecoveryKey)],
                }
            ])
        );
        
        policy
    }
}

impl AuthPolicy for ConfigurablePolicy {
    fn required_auth(&self, context: &AuthContext) -> Vec<AuthRequirement> {
        let requirement = self.operation_requirements
            .get(&context.operation)
            .unwrap_or(&self.default_requirement);
        
        vec![requirement.clone()]
    }
    
    fn validate(&self, credentials: &[AuthCredential], context: &AuthContext) -> AuthResult {
        let requirements = self.required_auth(context);
        
        // Use same validation logic as StandardAccountPolicy
        let standard_policy = StandardAccountPolicy;
        for requirement in &requirements {
            if !standard_policy.check_requirement(requirement, credentials, context) {
                return AuthResult::Failed(AuthError::InsufficientAuth);
            }
        }
        
        AuthResult::Success
    }
}

impl Default for ConfigurablePolicy {
    fn default() -> Self {
        Self::new()
    }
}