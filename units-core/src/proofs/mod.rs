pub mod engine;
pub mod hash_proof;

// Re-export the main types for convenience
pub use engine::{ProofEngine, SlotNumber, StateProof, UnitsObjectProof, VerificationResult};
pub use hash_proof::HashProofEngine;

// Helper functions
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current slot number based on system time
/// In a production system, this would use a synchronized clock
pub fn current_slot() -> SlotNumber {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    now
}
