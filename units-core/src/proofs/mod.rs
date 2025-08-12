pub mod engine;
pub mod merkle_proof;
pub mod proof_engine;

// Re-export the main types for convenience
pub use engine::{ProofEngine, SlotNumber, StateProof, UnitsObjectProof, VerificationResult};
// Re-export the concrete proof engine
pub use proof_engine::ProofEngine as ConcreteProofEngine;
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