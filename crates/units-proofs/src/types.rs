use serde::{Deserialize, Serialize};

/// Slot number type (represents points in time)
pub type SlotNumber = u64;

/// Object ID type for proofs (32-byte identifier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitsObjectId([u8; 32]);

impl UnitsObjectId {
    /// Create a new UnitsObjectId from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    /// Get the bytes of this ID
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::cmp::Ord for UnitsObjectId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::cmp::PartialOrd for UnitsObjectId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A cryptographic proof for a UNITS object
///
/// This proof commits to the state of a UnitsObject at a particular slot,
/// and optionally links to a previous proof to form a chain of state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitsObjectProof {
    /// The UnitsObjectId this proof is for
    pub object_id: UnitsObjectId,

    /// The slot number when this proof was created
    pub slot: SlotNumber,

    /// Hash of the object state this proof commits to
    pub object_hash: [u8; 32],

    /// Optional hash of the previous proof for this object
    /// If None, this is the first proof for the object
    pub prev_proof_hash: Option<[u8; 32]>,

    /// Optional hash of the transaction that led to this state change
    pub transaction_hash: Option<[u8; 32]>,

    /// Cryptographic data that authenticates this proof
    /// The format depends on the specific proof implementation
    pub proof_data: Vec<u8>,
}

impl UnitsObjectProof {
    /// Creates a new UnitsObjectProof with the given data
    pub fn new(
        object_id: UnitsObjectId,
        object_hash: [u8; 32],
        slot: SlotNumber,
        proof_data: Vec<u8>,
        prev_proof: Option<&UnitsObjectProof>,
        transaction_hash: Option<[u8; 32]>,
    ) -> Self {
        let prev_proof_hash = prev_proof.map(|p| p.hash());

        Self {
            object_id,
            slot,
            object_hash,
            prev_proof_hash,
            transaction_hash,
            proof_data,
        }
    }

    /// Computes the hash of this proof
    /// Used to link proofs in a chain
    pub fn hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.object_id.bytes());
        hasher.update(self.slot.to_le_bytes());
        hasher.update(self.object_hash);

        if let Some(prev_hash) = self.prev_proof_hash {
            hasher.update(prev_hash);
        }

        if let Some(tx_hash) = self.transaction_hash {
            hasher.update(tx_hash);
        }

        hasher.update(&self.proof_data);

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// A state proof represents the aggregated state of multiple objects at a specific slot
///
/// State proofs commit to the collective state of the system at a point in time,
/// and form a chain that can be used to verify the evolution of the system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProof {
    /// The slot number this state proof is for
    pub slot: SlotNumber,

    /// The hash of the previous state proof, if any
    pub prev_state_proof_hash: Option<[u8; 32]>,

    /// List of object IDs included in this state proof
    pub object_ids: Vec<UnitsObjectId>,

    /// Cryptographic data that authenticates this proof
    /// The format depends on the specific proof implementation
    pub proof_data: Vec<u8>,
}

impl StateProof {
    /// Creates a new StateProof with the given data
    pub fn new(
        slot: SlotNumber,
        proof_data: Vec<u8>,
        object_ids: Vec<UnitsObjectId>,
        prev_state_proof: Option<&StateProof>,
    ) -> Self {
        let prev_state_proof_hash = prev_state_proof.map(|p| p.hash());

        Self {
            slot,
            prev_state_proof_hash,
            object_ids,
            proof_data,
        }
    }

    /// Computes the hash of this state proof
    /// Used to link proofs in a chain
    pub fn hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.slot.to_le_bytes());

        if let Some(prev_hash) = self.prev_state_proof_hash {
            hasher.update(prev_hash);
        }

        // Hash all object IDs in a deterministic order
        let mut object_ids = self.object_ids.clone();
        object_ids.sort(); // Ensure deterministic ordering
        for id in object_ids {
            hasher.update(id.bytes());
        }

        hasher.update(&self.proof_data);

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Represents the result of verifying a proof chain
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// The proof chain is valid
    Valid,

    /// The proof chain is invalid for the specified reason
    Invalid(String),

    /// Missing data needed to complete verification
    MissingData(String),
}

/// Merkle tree node for inclusion proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub is_left: bool,
}

/// Storage error type for proof operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Proof verification failed")]
    ProofVerification,
    #[error("Proof not found")]
    ProofNotFound,
    #[error("Proof chain invalid: {0}")]
    ProofChainInvalid(String),
    #[error("Missing proof data: {0}")]
    ProofMissingData(String),
}