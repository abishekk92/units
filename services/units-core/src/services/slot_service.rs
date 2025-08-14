//! Slot service for managing time-based slot progression
//!
//! This service handles slot timing, transitions, and coordination
//! between transaction execution and proof generation.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast};
use tokio::time::{interval, MissedTickBehavior};

use units_core_types::{
    SlotNumber, TransactionReceipt, StateProof,
};

use crate::error::ServiceResult;
use super::transaction_service::TransactionService;
use super::proof_service::ProofService;

/// Slot transition event
#[derive(Debug, Clone)]
pub enum SlotEvent {
    /// New slot started
    SlotStarted {
        slot: SlotNumber,
        timestamp: u64,
    },
    /// Slot execution completed
    SlotExecuted {
        slot: SlotNumber,
        transaction_count: usize,
        success_count: usize,
    },
    /// Slot finalized with proof
    SlotFinalized {
        slot: SlotNumber,
        proof: StateProof,
    },
    /// Slot failed
    SlotFailed {
        slot: SlotNumber,
        error: String,
    },
}

/// Slot configuration
#[derive(Debug, Clone)]
pub struct SlotConfig {
    /// Duration of each slot in milliseconds
    pub slot_duration_ms: u64,
    /// Maximum transactions per slot
    pub max_transactions_per_slot: usize,
    /// Whether to auto-advance slots
    pub auto_advance: bool,
    /// Grace period for late transactions (ms)
    pub grace_period_ms: u64,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            slot_duration_ms: 1000, // 1 second slots
            max_transactions_per_slot: 1000,
            auto_advance: true,
            grace_period_ms: 100,
        }
    }
}

/// Slot state tracking
struct SlotState {
    current_slot: SlotNumber,
    slot_start_time: Instant,
    slot_start_timestamp: u64,
    receipts: Vec<TransactionReceipt>,
    finalized: bool,
}

/// Slot manager for coordinating slot transitions
pub struct SlotManager {
    config: SlotConfig,
    state: Arc<RwLock<SlotState>>,
    event_sender: broadcast::Sender<SlotEvent>,
    transaction_service: Arc<TransactionService>,
    proof_service: Arc<ProofService>,
}

impl SlotManager {
    pub fn new(
        config: SlotConfig,
        transaction_service: Arc<TransactionService>,
        proof_service: Arc<ProofService>,
    ) -> (Self, broadcast::Receiver<SlotEvent>) {
        let (event_sender, event_receiver) = broadcast::channel(100);
        
        let state = Arc::new(RwLock::new(SlotState {
            current_slot: 0,
            slot_start_time: Instant::now(),
            slot_start_timestamp: chrono::Utc::now().timestamp() as u64,
            receipts: Vec::new(),
            finalized: false,
        }));
        
        let manager = Self {
            config,
            state,
            event_sender,
            transaction_service,
            proof_service,
        };
        
        (manager, event_receiver)
    }

    /// Start automatic slot advancement
    pub async fn start_auto_advance(&self) -> ServiceResult<()> {
        if !self.config.auto_advance {
            return Ok(());
        }

        let state = self.state.clone();
        let config = self.config.clone();
        let event_sender = self.event_sender.clone();
        let transaction_service = self.transaction_service.clone();
        let proof_service = self.proof_service.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(config.slot_duration_ms));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;
                
                // Advance to next slot
                if let Err(e) = Self::advance_slot_internal(
                    &state,
                    &config,
                    &event_sender,
                    &transaction_service,
                    &proof_service,
                ).await {
                    log::error!("Failed to advance slot: {:?}", e);
                }
            }
        });

        Ok(())
    }

    /// Manually advance to next slot
    pub async fn advance_slot(&self) -> ServiceResult<SlotNumber> {
        Self::advance_slot_internal(
            &self.state,
            &self.config,
            &self.event_sender,
            &self.transaction_service,
            &self.proof_service,
        ).await
    }

    /// Internal slot advancement logic
    async fn advance_slot_internal(
        state: &Arc<RwLock<SlotState>>,
        config: &SlotConfig,
        event_sender: &broadcast::Sender<SlotEvent>,
        transaction_service: &Arc<TransactionService>,
        proof_service: &Arc<ProofService>,
    ) -> ServiceResult<SlotNumber> {
        // Finalize current slot
        {
            let mut slot_state = state.write().await;
            if !slot_state.finalized {
                let current = slot_state.current_slot;
                let receipts = slot_state.receipts.clone();
                
                // Generate and store proof
                match proof_service.finalize_slot(current, receipts).await {
                    Ok(proof) => {
                        let _ = event_sender.send(SlotEvent::SlotFinalized {
                            slot: current,
                            proof,
                        });
                        slot_state.finalized = true;
                    }
                    Err(e) => {
                        let _ = event_sender.send(SlotEvent::SlotFailed {
                            slot: current,
                            error: e.to_string(),
                        });
                        return Err(e);
                    }
                }
            }
        }

        // Start new slot
        let new_slot = {
            let mut slot_state = state.write().await;
            slot_state.current_slot += 1;
            slot_state.slot_start_time = Instant::now();
            slot_state.slot_start_timestamp = chrono::Utc::now().timestamp() as u64;
            slot_state.receipts.clear();
            slot_state.finalized = false;
            
            let new_slot = slot_state.current_slot;
            let timestamp = slot_state.slot_start_timestamp;
            
            // Notify about new slot
            let _ = event_sender.send(SlotEvent::SlotStarted {
                slot: new_slot,
                timestamp,
            });
            
            new_slot
        };

        // Update transaction service slot
        transaction_service.advance_slot().await?;

        // Execute pending transactions for new slot
        tokio::spawn({
            let state = state.clone();
            let event_sender = event_sender.clone();
            let transaction_service = transaction_service.clone();
            let config = config.clone();
            
            async move {
                // Wait for grace period to collect transactions
                tokio::time::sleep(Duration::from_millis(config.grace_period_ms)).await;
                
                // Execute transactions
                match transaction_service.execute_slot_transactions().await {
                    Ok(receipts) => {
                        let transaction_count = receipts.len();
                        let success_count = receipts.iter().filter(|r| r.success).count();
                        
                        // Store receipts
                        {
                            let mut slot_state = state.write().await;
                            slot_state.receipts = receipts;
                        }
                        
                        // Notify execution complete
                        let _ = event_sender.send(SlotEvent::SlotExecuted {
                            slot: new_slot,
                            transaction_count,
                            success_count,
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to execute slot transactions: {:?}", e);
                    }
                }
            }
        });

        Ok(new_slot)
    }

    /// Get current slot number
    pub async fn current_slot(&self) -> SlotNumber {
        self.state.read().await.current_slot
    }

    /// Get slot information
    pub async fn get_slot_info(&self) -> SlotInfo {
        let state = self.state.read().await;
        let elapsed = state.slot_start_time.elapsed();
        
        SlotInfo {
            current_slot: state.current_slot,
            slot_start_timestamp: state.slot_start_timestamp,
            elapsed_ms: elapsed.as_millis() as u64,
            remaining_ms: self.config.slot_duration_ms.saturating_sub(elapsed.as_millis() as u64),
            transaction_count: state.receipts.len(),
            finalized: state.finalized,
        }
    }

    /// Force finalize current slot
    pub async fn finalize_current_slot(&self) -> ServiceResult<StateProof> {
        let (slot, receipts) = {
            let state = self.state.read().await;
            (state.current_slot, state.receipts.clone())
        };

        let proof = self.proof_service.finalize_slot(slot, receipts).await?;
        
        {
            let mut state = self.state.write().await;
            state.finalized = true;
        }

        let _ = self.event_sender.send(SlotEvent::SlotFinalized {
            slot,
            proof: proof.clone(),
        });

        Ok(proof)
    }
}

#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub current_slot: SlotNumber,
    pub slot_start_timestamp: u64,
    pub elapsed_ms: u64,
    pub remaining_ms: u64,
    pub transaction_count: usize,
    pub finalized: bool,
}

/// Main slot service providing high-level slot management
pub struct SlotService {
    manager: Arc<SlotManager>,
    event_receiver: Arc<RwLock<broadcast::Receiver<SlotEvent>>>,
}

impl SlotService {
    pub fn new(
        config: SlotConfig,
        transaction_service: Arc<TransactionService>,
        proof_service: Arc<ProofService>,
    ) -> Self {
        let (manager, event_receiver) = SlotManager::new(
            config,
            transaction_service,
            proof_service,
        );
        
        Self {
            manager: Arc::new(manager),
            event_receiver: Arc::new(RwLock::new(event_receiver)),
        }
    }

    /// Start the slot service
    pub async fn start(&self) -> ServiceResult<()> {
        self.manager.start_auto_advance().await
    }

    /// Get current slot
    pub async fn current_slot(&self) -> SlotNumber {
        self.manager.current_slot().await
    }

    /// Get slot information
    pub async fn slot_info(&self) -> SlotInfo {
        self.manager.get_slot_info().await
    }

    /// Manually advance slot
    pub async fn advance_slot(&self) -> ServiceResult<SlotNumber> {
        self.manager.advance_slot().await
    }

    /// Force finalize current slot
    pub async fn finalize_current(&self) -> ServiceResult<StateProof> {
        self.manager.finalize_current_slot().await
    }

    /// Subscribe to slot events
    pub fn subscribe(&self) -> broadcast::Receiver<SlotEvent> {
        self.manager.event_sender.subscribe()
    }

    /// Wait for next slot event
    pub async fn next_event(&self) -> Option<SlotEvent> {
        let mut receiver = self.event_receiver.write().await;
        receiver.recv().await.ok()
    }

    /// Get slot statistics
    pub async fn get_stats(&self) -> SlotStats {
        let info = self.slot_info().await;
        
        SlotStats {
            current_slot: info.current_slot,
            slot_duration_ms: self.manager.config.slot_duration_ms,
            auto_advance: self.manager.config.auto_advance,
            max_transactions_per_slot: self.manager.config.max_transactions_per_slot,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlotStats {
    pub current_slot: SlotNumber,
    pub slot_duration_ms: u64,
    pub auto_advance: bool,
    pub max_transactions_per_slot: usize,
}