//! Proof engine implementation for UNITS
//! 
//! This module provides a hash-based proof engine that meets the core requirements:
//! 1. Cryptographically prove object state at any slot
//! 2. Cryptographically prove transaction inclusion in a slot

use units_core_types::{Proof, SlotNumber, StateProof, UnitsObjectProof, VerificationResult, MerkleNode, ProofStorageError, UnitsObjectId};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Proof engine using Blake3 hashing
#[derive(Debug, Clone, Default)]
pub struct ProofEngine;

impl ProofEngine {
    /// Create a new proof engine
    pub fn new() -> Self {
        Self
    }

    /// Generate a cryptographic proof for a UNITS object
    pub fn generate_object_proof<T: Proof>(
        &self,
        object: &T,
        prev_proof: Option<&UnitsObjectProof>,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, ProofStorageError> {
        // Get current slot
        let current_slot = crate::current_slot();
        
        // Compute object hash
        let object_hash = self.hash_object(object)?;
        
        // Create proof data as simple hash chain
        let proof_data = self.create_proof_data(
            &object_hash,
            prev_proof.map(|p| p.hash()),
            current_slot,
            transaction_hash,
        );
        
        Ok(UnitsObjectProof::new(
            object.id(),
            object_hash,
            current_slot,
            proof_data,
            prev_proof,
            transaction_hash,
        ))
    }

    /// Verify that a proof correctly commits to an object's state
    pub fn verify_object_proof<T: Proof>(
        &self,
        object: &T,
        proof: &UnitsObjectProof,
    ) -> Result<bool, ProofStorageError> {
        // Verify object ID matches
        if object.id() != proof.object_id {
            return Ok(false);
        }
        
        // Verify object hash
        let computed_hash = self.hash_object(object)?;
        if computed_hash != proof.object_hash {
            return Ok(false);
        }
        
        // Verify proof data integrity
        let expected_proof_data = self.create_proof_data(
            &proof.object_hash,
            proof.prev_proof_hash,
            proof.slot,
            proof.transaction_hash,
        );
        
        Ok(proof.proof_data == expected_proof_data)
    }

    /// Generate a state proof that includes transaction hashes for inclusion proofs
    pub fn generate_state_proof(
        &self,
        object_proofs: &[(UnitsObjectId, UnitsObjectProof)],
        transaction_hashes: &[[u8; 32]],
        prev_state_proof: Option<&StateProof>,
        slot: SlotNumber,
    ) -> Result<StateProof, ProofStorageError> {
        // Extract object IDs
        let object_ids: Vec<UnitsObjectId> = object_proofs
            .iter()
            .map(|(id, _)| *id)
            .collect();
        
        // Create state proof data
        let proof_data = StateProofData {
            object_root: self.compute_object_root(object_proofs)?,
            transaction_root: self.compute_transaction_root(transaction_hashes),
            slot,
        };
        
        let serialized = bincode::serialize(&proof_data)
            .map_err(|e| ProofStorageError::Serialization(e.to_string()))?;
        
        Ok(StateProof::new(
            slot,
            serialized,
            object_ids,
            prev_state_proof,
        ))
    }

    /// Verify that a state proof correctly commits to a collection of object proofs
    pub fn verify_state_proof(
        &self,
        state_proof: &StateProof,
        object_proofs: &[(UnitsObjectId, UnitsObjectProof)],
    ) -> Result<bool, ProofStorageError> {
        // Deserialize proof data
        let proof_data: StateProofData = bincode::deserialize(&state_proof.proof_data)
            .map_err(|e| ProofStorageError::Serialization(e.to_string()))?;
        
        // Compute expected object root
        let expected_root = self.compute_object_root(object_proofs)?;
        
        // Verify the root matches
        Ok(expected_root == proof_data.object_root && state_proof.slot == proof_data.slot)
    }

    /// Verify transaction inclusion in a state proof
    pub fn verify_transaction_inclusion(
        &self,
        state_proof: &StateProof,
        transaction_hash: &[u8; 32],
        _transaction_hashes: &[[u8; 32]],
        merkle_path: &[MerkleNode],
    ) -> Result<bool, ProofStorageError> {
        // Deserialize proof data
        let proof_data: StateProofData = bincode::deserialize(&state_proof.proof_data)
            .map_err(|e| ProofStorageError::Serialization(e.to_string()))?;
        
        // Verify the merkle path
        let computed_root = self.verify_merkle_path(transaction_hash, merkle_path)?;
        
        // Check if computed root matches the transaction root in state proof
        Ok(computed_root == proof_data.transaction_root)
    }

    // Helper methods

    fn hash_object<T: Proof>(&self, object: &T) -> Result<[u8; 32], ProofStorageError> {
        let serialized = bincode::serialize(object)
            .map_err(|e| ProofStorageError::Serialization(e.to_string()))?;
        
        let mut hasher = Hasher::new();
        hasher.update(&serialized);
        Ok(*hasher.finalize().as_bytes())
    }

    fn create_proof_data(
        &self,
        object_hash: &[u8; 32],
        prev_proof_hash: Option<[u8; 32]>,
        slot: SlotNumber,
        transaction_hash: Option<[u8; 32]>,
    ) -> Vec<u8> {
        let mut hasher = Hasher::new();
        hasher.update(object_hash);
        hasher.update(&slot.to_le_bytes());
        
        if let Some(prev_hash) = prev_proof_hash {
            hasher.update(&prev_hash);
        }
        
        if let Some(tx_hash) = transaction_hash {
            hasher.update(&tx_hash);
        }
        
        hasher.finalize().as_bytes().to_vec()
    }

    fn compute_object_root(&self, object_proofs: &[(UnitsObjectId, UnitsObjectProof)]) -> Result<[u8; 32], ProofStorageError> {
        let mut hasher = Hasher::new();
        
        // Sort by object ID for deterministic ordering
        let mut sorted_proofs = object_proofs.to_vec();
        sorted_proofs.sort_by_key(|(id, _)| *id);
        
        for (id, proof) in sorted_proofs {
            hasher.update(id.bytes());
            hasher.update(&proof.hash());
        }
        
        Ok(*hasher.finalize().as_bytes())
    }

    fn compute_transaction_root(&self, transaction_hashes: &[[u8; 32]]) -> [u8; 32] {
        if transaction_hashes.is_empty() {
            return [0u8; 32];
        }
        
        // Build simple merkle tree
        let mut current_level: Vec<[u8; 32]> = transaction_hashes.to_vec();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                let mut hasher = Hasher::new();
                hasher.update(&chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(&chunk[1]);
                } else {
                    hasher.update(&chunk[0]); // Duplicate if odd number
                }
                next_level.push(*hasher.finalize().as_bytes());
            }
            
            current_level = next_level;
        }
        
        current_level[0]
    }

    fn verify_merkle_path(&self, leaf: &[u8; 32], path: &[MerkleNode]) -> Result<[u8; 32], ProofStorageError> {
        let mut current_hash = *leaf;
        
        for node in path {
            let mut hasher = Hasher::new();
            if node.is_left {
                hasher.update(&node.hash);
                hasher.update(&current_hash);
            } else {
                hasher.update(&current_hash);
                hasher.update(&node.hash);
            }
            current_hash = *hasher.finalize().as_bytes();
        }
        
        Ok(current_hash)
    }

    /// Verify an entire history of proofs for an object
    pub fn verify_proof_history<T: Proof>(
        &self,
        object_states: &[(SlotNumber, T)],
        proofs: &[(SlotNumber, UnitsObjectProof)],
    ) -> VerificationResult {
        if object_states.is_empty() || proofs.is_empty() {
            return VerificationResult::MissingData(
                "No object states or proofs provided".to_string(),
            );
        }

        // Verify each state has a corresponding proof
        for (slot, obj) in object_states {
            let matching_proof = proofs.iter().find(|(proof_slot, _)| proof_slot == slot);

            if let Some((_, proof)) = matching_proof {
                match self.verify_object_proof(obj, proof) {
                    Ok(true) => {}
                    Ok(false) => {
                        return VerificationResult::Invalid(format!(
                            "Proof verification failed for slot {}",
                            slot
                        ))
                    }
                    Err(e) => {
                        return VerificationResult::Invalid(format!(
                            "Proof verification error at slot {}: {}",
                            slot, e
                        ))
                    }
                }
            } else {
                return VerificationResult::MissingData(format!("Missing proof for slot {}", slot));
            }
        }

        // Verify proof chain links
        for i in 1..proofs.len() {
            let (current_slot, current_proof) = &proofs[i];
            let (prev_slot, prev_proof) = &proofs[i - 1];

            if let Some(prev_hash) = &current_proof.prev_proof_hash {
                let computed_hash = prev_proof.hash();
                if computed_hash != *prev_hash {
                    return VerificationResult::Invalid(format!(
                        "Proof chain broken between slots {} and {}",
                        prev_slot, current_slot
                    ));
                }
            } else {
                return VerificationResult::Invalid(format!(
                    "Proof at slot {} does not reference previous proof",
                    current_slot
                ));
            }
        }

        VerificationResult::Valid
    }
}

/// State proof data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateProofData {
    object_root: [u8; 32],
    transaction_root: [u8; 32],
    slot: SlotNumber,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestObject {
        id: UnitsObjectId,
        data: Vec<u8>,
    }

    impl Proof for TestObject {
        fn id(&self) -> UnitsObjectId {
            self.id
        }
    }

    #[test]
    fn test_object_proof_generation_and_verification() {
        let engine = ProofEngine::new();
        
        // Create test object
        let object_id = UnitsObjectId::from_bytes([1u8; 32]);
        let object = TestObject {
            id: object_id,
            data: vec![1, 2, 3],
        };
        
        // Generate proof
        let proof = engine.generate_object_proof(&object, None, None).unwrap();
        
        // Verify proof
        assert!(engine.verify_object_proof(&object, &proof).unwrap());
        
        // Verify with wrong object fails
        let wrong_object = TestObject {
            id: UnitsObjectId::from_bytes([2u8; 32]),
            data: vec![4, 5, 6],
        };
        assert!(!engine.verify_object_proof(&wrong_object, &proof).unwrap());
    }

    #[test]
    fn test_proof_chain() {
        let engine = ProofEngine::new();
        
        let object_id = UnitsObjectId::from_bytes([1u8; 32]);
        let mut object = TestObject {
            id: object_id,
            data: vec![1, 2, 3],
        };
        
        // Generate first proof
        let proof1 = engine.generate_object_proof(&object, None, None).unwrap();
        
        // Modify object and generate second proof
        object.data = vec![4, 5, 6];
        let proof2 = engine.generate_object_proof(&object, Some(&proof1), None).unwrap();
        
        // Verify chain
        assert_eq!(proof2.prev_proof_hash, Some(proof1.hash()));
    }
}