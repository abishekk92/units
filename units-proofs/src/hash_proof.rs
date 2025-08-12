use crate::engine::{ProofEngine, SlotNumber, StateProof, UnitsObjectProof};
use blake3;
use units_core::error::StorageError;
use units_core::id::UnitsObjectId;
use units_core::objects::UnitsObject;

/// A hash-based proof engine that creates proofs using BLAKE3 hashes
/// This provides a straightforward implementation without complex data structures
#[derive(Debug, Clone)]
pub struct HashProofEngine;

impl HashProofEngine {
    pub fn new() -> Self {
        Self
    }
}

impl ProofEngine for HashProofEngine {
    fn generate_object_proof(
        &self,
        object: &UnitsObject,
        prev_proof: Option<&UnitsObjectProof>,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<UnitsObjectProof, StorageError> {
        // Create a simple hash-based proof
        let object_hash: [u8; 32] = blake3::hash(object.data()).into();
        let prev_proof_hash = prev_proof.map(|p| p.hash());
        
        // Create proof data that's just the object hash
        let proof_data = object_hash.to_vec();
        
        // Use current time as slot number (simplified)
        let slot = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(UnitsObjectProof {
            object_id: *object.id(),
            slot,
            object_hash,
            prev_proof_hash,
            transaction_hash,
            proof_data,
        })
    }

    fn verify_object_proof(
        &self,
        object: &UnitsObject,
        proof: &UnitsObjectProof,
    ) -> Result<bool, StorageError> {
        // Simple verification: check that the object hash matches
        let expected_hash: [u8; 32] = blake3::hash(object.data()).into();
        Ok(expected_hash == proof.object_hash)
    }

    fn verify_proof_chain(
        &self,
        _object: &UnitsObject,
        proof: &UnitsObjectProof,
        prev_proof: &UnitsObjectProof,
    ) -> Result<bool, StorageError> {
        // Simple verification: check that the current proof references the previous one
        Ok(proof.prev_proof_hash == Some(prev_proof.hash()))
    }

    fn generate_state_proof(
        &self,
        object_proofs: &[(UnitsObjectId, UnitsObjectProof)],
        prev_state_proof: Option<&StateProof>,
        slot: SlotNumber,
    ) -> Result<StateProof, StorageError> {
        // Create a simple state proof by hashing all object IDs
        let mut object_ids: Vec<_> = object_proofs.iter().map(|(id, _)| *id).collect();
        object_ids.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));

        // Simple proof data is just concatenated hashes
        let mut proof_data = Vec::new();
        for (id, _) in object_proofs {
            proof_data.extend_from_slice(id.as_ref());
        }
        let state_hash: [u8; 32] = blake3::hash(&proof_data).into();

        Ok(StateProof {
            slot,
            prev_state_proof_hash: prev_state_proof.map(|p| p.hash()),
            object_ids,
            proof_data: state_hash.to_vec(),
        })
    }

    fn verify_state_proof(
        &self,
        state_proof: &StateProof,
        object_proofs: &[(UnitsObjectId, UnitsObjectProof)],
    ) -> Result<bool, StorageError> {
        // Simple verification: check that object IDs match
        let mut proof_object_ids: Vec<_> = object_proofs.iter().map(|(id, _)| *id).collect();
        proof_object_ids.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));

        Ok(proof_object_ids == state_proof.object_ids)
    }

    fn verify_state_proof_chain(
        &self,
        state_proof: &StateProof,
        prev_state_proof: &StateProof,
    ) -> Result<bool, StorageError> {
        // Simple verification: check that the current state proof references the previous one
        Ok(state_proof.prev_state_proof_hash == Some(prev_state_proof.hash()))
    }
}