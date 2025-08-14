//! Verification trait definitions for the UNITS system
//!
//! This module defines the verification interfaces that runtime implementations must provide.

use crate::id::UnitsObjectId;
use crate::objects::UnitsObject;
use crate::transaction::TransactionReceipt;
use crate::{SlotNumber, StateProof, UnitsObjectProof, VerificationResult};
use std::collections::HashMap;

/// Core verification capabilities that all runtimes must provide
/// 
/// This trait ensures that runtime implementations include proper verification
/// of proofs, receipts, and transaction integrity.
pub trait Verifier: Send + Sync {
    //--------------------------------------------------------------------------
    // PROOF VERIFICATION
    //--------------------------------------------------------------------------
    
    /// Verify a single object proof against its current state
    /// 
    /// # Parameters
    /// * `object` - The object to verify against
    /// * `proof` - The cryptographic proof to verify
    /// 
    /// # Returns
    /// A VerificationResult indicating whether the proof is valid
    fn verify_object_proof(
        &self,
        object: &UnitsObject,
        proof: &UnitsObjectProof,
    ) -> VerificationResult;
    
    /// Verify a chain of proofs for an object across multiple slots
    /// 
    /// # Parameters  
    /// * `object_states` - Historical object states in slot order
    /// * `proofs` - Corresponding proofs in slot order
    /// 
    /// # Returns
    /// A VerificationResult indicating whether the proof chain is valid
    fn verify_proof_chain(
        &self,
        object_states: &[(SlotNumber, UnitsObject)],
        proofs: &[(SlotNumber, UnitsObjectProof)],
    ) -> VerificationResult;
    
    /// Verify a state proof for multiple objects
    /// 
    /// # Parameters
    /// * `state_proof` - The aggregated state proof to verify
    /// * `object_proofs` - Individual object proofs included in the state proof
    /// 
    /// # Returns
    /// A VerificationResult indicating whether the state proof is valid
    fn verify_state_proof(
        &self,
        state_proof: &StateProof,
        object_proofs: &HashMap<UnitsObjectId, UnitsObjectProof>,
    ) -> VerificationResult;
    
    //--------------------------------------------------------------------------
    // TRANSACTION VERIFICATION
    //--------------------------------------------------------------------------
    
    /// Verify the integrity of a transaction receipt
    /// 
    /// # Parameters
    /// * `receipt` - The transaction receipt to verify
    /// * `objects` - Current state of objects referenced in the receipt
    /// 
    /// # Returns
    /// A VerificationResult indicating whether the receipt is valid
    fn verify_transaction_receipt(
        &self,
        receipt: &TransactionReceipt,
        objects: &HashMap<UnitsObjectId, UnitsObject>,
    ) -> VerificationResult;
    
    /// Verify that a transaction is included in a collection of receipts
    /// 
    /// # Parameters
    /// * `transaction_hash` - Hash of the transaction to find
    /// * `receipts` - Collection of receipts to search
    /// 
    /// # Returns
    /// A VerificationResult indicating whether the transaction was found
    fn verify_transaction_included(
        &self,
        transaction_hash: &[u8; 32],
        receipts: &[TransactionReceipt],
    ) -> VerificationResult;
    
    //--------------------------------------------------------------------------
    // INTEGRITY CHECKS
    //--------------------------------------------------------------------------
    
    /// Detect double-spend attempts for a specific object
    /// 
    /// A double-spend occurs when the same object is modified by multiple
    /// transactions within the same slot.
    /// 
    /// # Parameters
    /// * `object_id` - ID of the object to check
    /// * `receipts` - Collection of transaction receipts to analyze
    /// 
    /// # Returns
    /// A VerificationResult indicating whether a double-spend was detected
    fn detect_double_spend(
        &self,
        object_id: &UnitsObjectId,
        receipts: &[TransactionReceipt],
    ) -> VerificationResult;
}