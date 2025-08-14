#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{vec::Vec, string::String, collections::BTreeMap};

use super::{Authenticator, AuthCredential, AuthContext, AuthResult, AuthError, AuthFactor};
use units_kernel_sdk::UnitsObjectId;
use borsh::{BorshDeserialize, BorshSerialize};

/// Time-based One-Time Password (TOTP) authenticator
pub struct TotpAuthenticator {
    /// Map of account ID to TOTP secret
    secrets: BTreeMap<UnitsObjectId, TotpSecret>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TotpSecret {
    pub secret: Vec<u8>,
    pub digits: u32,      // Usually 6 or 8
    pub period: u64,      // Usually 30 seconds
    pub algorithm: TotpAlgorithm,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum TotpAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl TotpAuthenticator {
    pub fn new() -> Self {
        Self {
            secrets: BTreeMap::new(),
        }
    }
    
    /// Add TOTP secret for an account
    pub fn add_secret(&mut self, account_id: UnitsObjectId, secret: TotpSecret) {
        self.secrets.insert(account_id, secret);
    }
    
    /// Remove TOTP secret for an account
    pub fn remove_secret(&mut self, account_id: &UnitsObjectId) {
        self.secrets.remove(account_id);
    }
    
    /// Verify TOTP code
    fn verify_totp(&self, account_id: &UnitsObjectId, code: &str, timestamp: u64) -> bool {
        let secret = match self.secrets.get(account_id) {
            Some(s) => s,
            None => return false,
        };
        
        // Calculate time step
        let time_step = timestamp / secret.period;
        
        // Check current time step and adjacent ones (to handle clock skew)
        for step in [time_step.saturating_sub(1), time_step, time_step + 1] {
            let expected_code = self.generate_totp_code(secret, step);
            if expected_code == code {
                return true;
            }
        }
        
        false
    }
    
    fn generate_totp_code(&self, secret: &TotpSecret, time_step: u64) -> String {
        // This is a simplified TOTP implementation
        // In production, you'd use a proper TOTP library like `totp-lite`
        
        let time_bytes = time_step.to_be_bytes();
        let hash = match secret.algorithm {
            TotpAlgorithm::Sha1 => {
                // Would use HMAC-SHA1
                self.simple_hash(&secret.secret, &time_bytes)
            }
            TotpAlgorithm::Sha256 => {
                // Would use HMAC-SHA256
                self.simple_hash(&secret.secret, &time_bytes)
            }
            TotpAlgorithm::Sha512 => {
                // Would use HMAC-SHA512
                self.simple_hash(&secret.secret, &time_bytes)
            }
        };
        
        // Dynamic truncation
        let offset = (hash[hash.len() - 1] & 0x0f) as usize;
        let code = ((hash[offset] as u32 & 0x7f) << 24)
            | ((hash[offset + 1] as u32 & 0xff) << 16)
            | ((hash[offset + 2] as u32 & 0xff) << 8)
            | (hash[offset + 3] as u32 & 0xff);
        
        let code = code % 10_u32.pow(secret.digits);
        format!("{:0width$}", code, width = secret.digits as usize)
    }
    
    fn simple_hash(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        // Simplified hash for demo - in production use proper HMAC
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(data);
        hasher.finalize().to_vec()
    }
}

impl Authenticator for TotpAuthenticator {
    fn verify(&self, credential: &AuthCredential, context: &AuthContext) -> AuthResult {
        if let AuthCredential::TimeBasedCode { code, timestamp } = credential {
            if self.verify_totp(&context.target_account, code, *timestamp) {
                AuthResult::Success
            } else {
                AuthResult::Failed(AuthError::InvalidCredentials)
            }
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        vec![AuthFactor::TimeBasedCode]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        matches!(credential, AuthCredential::TimeBasedCode { .. })
    }
}

impl Default for TotpAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

/// Hardware token authenticator (FIDO2/WebAuthn style)
pub struct HardwareTokenAuthenticator {
    /// Map of token ID to token info
    registered_tokens: BTreeMap<String, TokenInfo>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TokenInfo {
    pub token_id: String,
    pub account_id: UnitsObjectId,
    pub public_key: Vec<u8>,
    pub credential_id: Vec<u8>,
}

impl HardwareTokenAuthenticator {
    pub fn new() -> Self {
        Self {
            registered_tokens: BTreeMap::new(),
        }
    }
    
    /// Register a hardware token for an account
    pub fn register_token(&mut self, token_info: TokenInfo) {
        self.registered_tokens.insert(token_info.token_id.clone(), token_info);
    }
    
    /// Remove a hardware token
    pub fn remove_token(&mut self, token_id: &str) {
        self.registered_tokens.remove(token_id);
    }
}

impl Authenticator for HardwareTokenAuthenticator {
    fn verify(&self, credential: &AuthCredential, context: &AuthContext) -> AuthResult {
        if let AuthCredential::HardwareToken { token_id, challenge_response } = credential {
            let token_info = match self.registered_tokens.get(token_id) {
                Some(info) => info,
                None => return AuthResult::Failed(AuthError::InvalidCredentials),
            };
            
            // Verify the token belongs to the target account
            if token_info.account_id != context.target_account {
                return AuthResult::Failed(AuthError::InvalidCredentials);
            }
            
            // In a real implementation, you would:
            // 1. Verify the challenge_response using the token's public key
            // 2. Check that the challenge includes the operation context
            // 3. Validate the signature against the registered credential
            
            // For now, just check that we have a response
            if challenge_response.is_empty() {
                AuthResult::Failed(AuthError::InvalidCredentials)
            } else {
                // TODO: Implement proper FIDO2/WebAuthn verification
                AuthResult::Success
            }
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        vec![AuthFactor::HardwareToken]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        matches!(credential, AuthCredential::HardwareToken { .. })
    }
}

impl Default for HardwareTokenAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

/// Biometric authenticator
pub struct BiometricAuthenticator {
    /// Map of account ID to enrolled biometric templates
    enrolled_biometrics: BTreeMap<UnitsObjectId, Vec<BiometricTemplate>>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BiometricTemplate {
    pub biometric_type: String, // "fingerprint", "face", "voice", etc.
    pub template_hash: Vec<u8>,
    pub enrollment_date: u64,
}

impl BiometricAuthenticator {
    pub fn new() -> Self {
        Self {
            enrolled_biometrics: BTreeMap::new(),
        }
    }
    
    /// Enroll a biometric template for an account
    pub fn enroll_biometric(&mut self, account_id: UnitsObjectId, template: BiometricTemplate) {
        self.enrolled_biometrics
            .entry(account_id)
            .or_insert_with(Vec::new)
            .push(template);
    }
    
    /// Remove a biometric template
    pub fn remove_biometric(&mut self, account_id: &UnitsObjectId, biometric_type: &str) {
        if let Some(templates) = self.enrolled_biometrics.get_mut(account_id) {
            templates.retain(|t| t.biometric_type != biometric_type);
        }
    }
}

impl Authenticator for BiometricAuthenticator {
    fn verify(&self, credential: &AuthCredential, context: &AuthContext) -> AuthResult {
        if let AuthCredential::Biometric { biometric_type, template_hash } = credential {
            let templates = match self.enrolled_biometrics.get(&context.target_account) {
                Some(t) => t,
                None => return AuthResult::Failed(AuthError::InvalidCredentials),
            };
            
            // Check if any enrolled template matches
            for template in templates {
                if template.biometric_type == *biometric_type && template.template_hash == *template_hash {
                    return AuthResult::Success;
                }
            }
            
            AuthResult::Failed(AuthError::InvalidCredentials)
        } else {
            AuthResult::Failed(AuthError::UnsupportedMethod)
        }
    }
    
    fn supported_factors(&self) -> Vec<AuthFactor> {
        vec![AuthFactor::Biometric]
    }
    
    fn can_handle(&self, credential: &AuthCredential) -> bool {
        matches!(credential, AuthCredential::Biometric { .. })
    }
}

impl Default for BiometricAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create all default multi-factor authenticators
pub fn create_default_mfa_authenticators() -> Vec<Box<dyn Authenticator>> {
    vec![
        Box::new(TotpAuthenticator::new()),
        Box::new(HardwareTokenAuthenticator::new()),
        Box::new(BiometricAuthenticator::new()),
    ]
}