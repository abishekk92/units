pub mod engine;
pub mod types;

// Re-export main types and functions for convenience
pub use engine::ProofEngine;
pub use types::{SlotNumber, StateProof, UnitsObjectProof, VerificationResult, MerkleNode};

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