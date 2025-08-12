//! Concrete proof engine implementation for UNITS
//! 
//! This module provides a concrete Merkle-based proof engine, eliminating the
//! need for the ProofEngine trait abstraction.

use crate::error::StorageError;
use crate::objects::UnitsObject;
use crate::proofs::{SlotNumber, StateProof, UnitsObjectProof, VerificationResult};
use blake3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//==============================================================================
// PROOF ENGINE IMPLEMENTATION
//==============================================================================

/// Concrete proof engine using Merkle trees
/// 
/// This replaces the ProofEngine trait with a concrete implementation,
/// simplifying the codebase by removing unnecessary abstraction.
#[derive(Debug, Clone, Default)]
pub struct ProofEngine {
    /// Current slot number
    current_slot: SlotNumber,
}

impl ProofEngine {
    /// Create a new proof engine
    pub fn new() -> Self {
        Self {
            current_slot: crate::proofs::current_slot(),
        }
    }
    
    /// Set the current slot (for testing)
    pub fn set_slot(&mut self, slot: SlotNumber) {
        self.current_slot = slot;
    }
    
    /// Get the current slot
    pub fn current_slot(&self) -> SlotNumber {
        self.current_slot
    }
    
    //--------------------------------------------------------------------------
    // OBJECT PROOF GENERATION
    //--------------------------------------------------------------------------
    
    /// Generate a cryptographic proof for a UNITS object
    pub fn generate_object_proof(
        &self,
        object: &UnitsObject,
        prev_proof: Option<&UnitsObjectProof>,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError> {
        // Compute object hash
        let object_hash = Self::hash_object(object);
        
        // Create proof data
        let proof_data = MerkleProofData {
            object_hash,
            prev_proof_hash: prev_proof.map(|p| p.hash()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        // Serialize proof data
        let serialized = bincode::serialize(&proof_data)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        Ok(UnitsObjectProof {
            object_id: *object.id(),
            slot: self.current_slot,
            object_hash,
            prev_proof_hash: proof_data.prev_proof_hash,
            transaction_hash,
            proof_data: serialized,
        })
    }
    
    /// Verify that a proof correctly commits to an object's state
    pub fn verify_object_proof(
        &self,
        object: &UnitsObject,
        proof: &UnitsObjectProof,
    ) -> VerificationResult {
        // Check object ID matches
        if object.id() != &proof.object_id {
            return VerificationResult::Invalid(
                "Object ID mismatch".to_string()
            );
        }
        
        // Verify object hash
        let computed_hash = Self::hash_object(object);
        if computed_hash != proof.object_hash {
            return VerificationResult::Invalid(
                "Object hash mismatch".to_string()
            );
        }
        
        // Deserialize and verify proof data
        match bincode::deserialize::<MerkleProofData>(&proof.proof_data) {
            Ok(proof_data) => {
                if proof_data.object_hash != computed_hash {
                    return VerificationResult::Invalid(
                        "Proof data hash mismatch".to_string()
                    );
                }
                
                if proof_data.prev_proof_hash != proof.prev_proof_hash {
                    return VerificationResult::Invalid(
                        "Previous proof hash mismatch".to_string()
                    );
                }
                
                VerificationResult::Valid
            }
            Err(e) => VerificationResult::Invalid(
                format!("Failed to deserialize proof data: {}", e)
            ),
        }
    }
    
    /// Verify a chain of proofs
    pub fn verify_proof_chain(
        &self,
        prev_object: &UnitsObject,
        prev_proof: &UnitsObjectProof,
        curr_object: &UnitsObject,
        curr_proof: &UnitsObjectProof,
    ) -> VerificationResult {
        // Verify previous proof
        match self.verify_object_proof(prev_object, prev_proof) {
            VerificationResult::Valid => {}
            result => return result,
        }
        
        // Verify current proof
        match self.verify_object_proof(curr_object, curr_proof) {
            VerificationResult::Valid => {}
            result => return result,
        }
        
        // Verify chain linkage
        if curr_proof.prev_proof_hash != Some(prev_proof.hash()) {
            return VerificationResult::Invalid(
                "Proof chain linkage broken".to_string()
            );
        }
        
        // Verify slot ordering
        if curr_proof.slot <= prev_proof.slot {
            return VerificationResult::Invalid(
                "Invalid slot ordering in proof chain".to_string()
            );
        }
        
        VerificationResult::Valid
    }
    
    /// Verify an entire proof history
    pub fn verify_proof_history(
        &self,
        object_states: &[(SlotNumber, UnitsObject)],
        proofs: &[(SlotNumber, UnitsObjectProof)],
    ) -> VerificationResult {
        if object_states.len() != proofs.len() {
            return VerificationResult::Invalid(
                "Mismatched object states and proofs".to_string()
            );
        }
        
        if object_states.is_empty() {
            return VerificationResult::Valid;
        }
        
        // Verify each proof corresponds to its object state
        for i in 0..object_states.len() {
            let (obj_slot, object) = &object_states[i];
            let (proof_slot, proof) = &proofs[i];
            
            if obj_slot != proof_slot {
                return VerificationResult::Invalid(
                    format!("Slot mismatch at index {}", i)
                );
            }
            
            match self.verify_object_proof(object, proof) {
                VerificationResult::Valid => {}
                result => return result,
            }
            
            // Verify chain linkage (except for first proof)
            if i > 0 {
                let (_, prev_proof) = &proofs[i - 1];
                if proof.prev_proof_hash != Some(prev_proof.hash()) {
                    return VerificationResult::Invalid(
                        format!("Broken chain linkage at index {}", i)
                    );
                }
            }
        }
        
        VerificationResult::Valid
    }
    
    //--------------------------------------------------------------------------
    // STATE PROOF GENERATION
    //--------------------------------------------------------------------------
    
    /// Generate a state proof for multiple objects
    pub fn generate_state_proof(
        &self,
        objects: &[UnitsObject],
        prev_state_proof: Option<&StateProof>,
    ) -> Result<StateProof, StorageError> {
        // Extract object IDs
        let object_ids: Vec<_> = objects.iter().map(|o| *o.id()).collect();
        
        // Create Merkle tree from objects
        let merkle_root = self.compute_merkle_root(objects)?;
        
        // Create proof data
        let proof_data = StateProofData {
            merkle_root,
            object_count: objects.len() as u64,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        // Serialize proof data
        let serialized = bincode::serialize(&proof_data)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        Ok(StateProof {
            slot: self.current_slot,
            prev_state_proof_hash: prev_state_proof.map(|p| p.hash()),
            object_ids,
            proof_data: serialized,
        })
    }
    
    /// Verify a state proof
    pub fn verify_state_proof(
        &self,
        objects: &[UnitsObject],
        proof: &StateProof,
    ) -> VerificationResult {
        // Verify object count matches
        if objects.len() != proof.object_ids.len() {
            return VerificationResult::Invalid(
                "Object count mismatch".to_string()
            );
        }
        
        // Verify all object IDs are present
        let object_map: HashMap<_, _> = objects.iter()
            .map(|o| (*o.id(), o))
            .collect();
        
        for id in &proof.object_ids {
            if !object_map.contains_key(id) {
                return VerificationResult::MissingData(
                    format!("Missing object: {:?}", id)
                );
            }
        }
        
        // Deserialize proof data
        let proof_data = match bincode::deserialize::<StateProofData>(&proof.proof_data) {
            Ok(data) => data,
            Err(e) => return VerificationResult::Invalid(
                format!("Failed to deserialize proof data: {}", e)
            ),
        };
        
        // Verify Merkle root
        let computed_root = match self.compute_merkle_root(objects) {
            Ok(root) => root,
            Err(e) => return VerificationResult::Invalid(
                format!("Failed to compute Merkle root: {}", e)
            ),
        };
        
        if computed_root != proof_data.merkle_root {
            return VerificationResult::Invalid(
                "Merkle root mismatch".to_string()
            );
        }
        
        VerificationResult::Valid
    }
    
    //--------------------------------------------------------------------------
    // HELPER METHODS
    //--------------------------------------------------------------------------
    
    /// Hash a UNITS object
    fn hash_object(object: &UnitsObject) -> [u8; 32] {
        let serialized = bincode::serialize(object).unwrap_or_default();
        blake3::hash(&serialized).into()
    }
    
    /// Compute Merkle root for a set of objects
    fn compute_merkle_root(&self, objects: &[UnitsObject]) -> Result<[u8; 32], StorageError> {
        if objects.is_empty() {
            return Ok([0u8; 32]);
        }
        
        // Sort objects by ID for deterministic ordering
        let mut sorted_objects = objects.to_vec();
        sorted_objects.sort_by_key(|o| *o.id());
        
        // Compute leaf hashes
        let mut hashes: Vec<[u8; 32]> = sorted_objects.iter()
            .map(Self::hash_object)
            .collect();
        
        // Build Merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(&chunk[0]);
                    hasher.update(&chunk[1]);
                    hasher.finalize().into()
                } else {
                    chunk[0]
                };
                next_level.push(combined);
            }
            
            hashes = next_level;
        }
        
        Ok(hashes[0])
    }
}

//==============================================================================
// PROOF DATA STRUCTURES
//==============================================================================

/// Data stored in object proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MerkleProofData {
    /// Hash of the object
    object_hash: [u8; 32],
    
    /// Hash of the previous proof
    prev_proof_hash: Option<[u8; 32]>,
    
    /// Timestamp when proof was created
    timestamp: u64,
}

/// Data stored in state proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateProofData {
    /// Merkle root of all objects
    merkle_root: [u8; 32],
    
    /// Number of objects in the state
    object_count: u64,
    
    /// Timestamp when proof was created
    timestamp: u64,
}