//! Mesh Consensus Engine
//!
//! Core BFT consensus engine for local mesh blockchain networks.
//! Coordinates validator participation, block proposals, and finalization.

use super::types::*;
use super::validator::{ValidatorSet, MeshValidator, ValidatorStatus};
use super::round::RoundManager;
use super::config::*;

use anyhow::{Result, anyhow};
use tracing::{info, warn, debug, error};
use std::collections::HashMap;

/// Main consensus engine for mesh networks
pub struct MeshConsensusEngine {
    /// Mesh ID this consensus is for
    mesh_id: [u8; 32],
    
    /// This node's validator ID (if validator)
    our_validator_id: Option<[u8; 32]>,
    
    /// Set of validators
    validators: ValidatorSet,
    
    /// Current consensus state
    state: ConsensusState,
    
    /// Current round manager
    round_manager: Option<RoundManager>,
    
    /// Received messages for current height
    message_buffer: HashMap<u64, Vec<ConsensusMessage>>,
    
    /// Maximum rounds before giving up
    max_rounds: u64,
}

impl MeshConsensusEngine {
    /// Create new consensus engine
    pub fn new(
        mesh_id: [u8; 32],
        genesis_hash: [u8; 32],
        our_validator_id: Option<[u8; 32]>,
    ) -> Self {
        Self {
            mesh_id,
            our_validator_id,
            validators: ValidatorSet::new(),
            state: ConsensusState::new(genesis_hash),
            round_manager: None,
            message_buffer: HashMap::new(),
            max_rounds: MAX_ROUNDS_PER_HEIGHT,
        }
    }
    
    /// Add a validator to the consensus
    pub fn add_validator(&mut self, validator: MeshValidator) {
        info!(
            "Adding validator {} to mesh consensus",
            hex::encode(&validator.validator_id)
        );
        self.validators.add_validator(validator);
        self.state.validator_count = self.validators.active_count();
    }
    
    /// Remove a validator
    pub fn remove_validator(&mut self, validator_id: &[u8; 32]) {
        info!(
            "Removing validator {} from mesh consensus",
            hex::encode(validator_id)
        );
        self.validators.remove_validator(validator_id);
        self.state.validator_count = self.validators.active_count();
    }
    
    /// Start consensus for a new block height
    pub fn start_new_height(&mut self, height: u64) -> Result<()> {
        if self.validators.active_count() < MIN_VALIDATORS {
            return Err(anyhow!(
                "Not enough validators for consensus: {} < {}",
                self.validators.active_count(),
                MIN_VALIDATORS
            ));
        }
        
        // Get proposer for this height
        let proposer = self.validators.get_proposer()
            .ok_or_else(|| anyhow!("No proposer available"))?;
        
        info!(
            "Starting consensus for height {}, proposer: {}",
            height,
            hex::encode(&proposer)
        );
        
        // Update state
        self.state.start_new_height(height, proposer);
        self.state.is_proposer = Some(proposer) == self.our_validator_id;
        
        // Create round manager
        self.round_manager = Some(RoundManager::new(
            height,
            0,
            proposer,
            self.validators.active_count(),
        ));
        
        // Clear old message buffer
        self.message_buffer.retain(|&h, _| h >= height);
        
        Ok(())
    }
    
    /// Process a consensus message from another validator
    pub fn process_message(&mut self, message: ConsensusMessage) -> Result<()> {
        // Verify message is for current height
        if message.height != self.state.height {
            debug!(
                "Buffering message for height {} (current: {})",
                message.height, self.state.height
            );
            self.message_buffer
                .entry(message.height)
                .or_insert_with(Vec::new)
                .push(message);
            return Ok(());
        }
        
        // Store validator_id before message is moved
        let validator_id = message.validator_id;
        
        // Verify sender is a validator
        let validator = self.validators.get_validator(&validator_id)
            .ok_or_else(|| anyhow!("Message from unknown validator"))?;
        
        if validator.status != ValidatorStatus::Active {
            return Err(anyhow!("Message from inactive validator"));
        }
        
        // Process based on message type
        match message.message_type {
            ConsensusMessageType::Proposal => self.process_proposal(message)?,
            ConsensusMessageType::Prevote => self.process_prevote(message)?,
            ConsensusMessageType::Precommit => self.process_precommit(message)?,
            ConsensusMessageType::Commit => self.process_commit(message)?,
        }
        
        // Update validator last seen
        if let Some(validator) = self.validators.get_validator_mut(&validator_id) {
            validator.mark_online();
        }
        
        Ok(())
    }
    
    /// Process a proposal message
    fn process_proposal(&mut self, message: ConsensusMessage) -> Result<()> {
        let manager = self.round_manager.as_mut()
            .ok_or_else(|| anyhow!("No active round"))?;
        
        // Verify it's from the expected proposer
        if message.validator_id != manager.get_round().proposer {
            return Err(anyhow!("Proposal from non-proposer"));
        }
        
        // Extract proposal payload
        let proposal = match message.payload {
            MessagePayload::Proposal(p) => p,
            _ => return Err(anyhow!("Invalid payload for proposal message")),
        };
        
        info!(
            "Received proposal for height {}, round {}, block: {}",
            message.height,
            message.round,
            hex::encode(&proposal.block_hash[..8])
        );
        
        // Store proposal in round
        manager.get_round_mut().proposal = Some(proposal);
        
        // Record proposal for validator
        if let Some(validator) = self.validators.get_validator_mut(&message.validator_id) {
            validator.record_proposal();
        }
        
        // Try to advance to prevote phase
        manager.try_advance_phase();
        
        Ok(())
    }
    
    /// Process a prevote message
    fn process_prevote(&mut self, message: ConsensusMessage) -> Result<()> {
        let manager = self.round_manager.as_mut()
            .ok_or_else(|| anyhow!("No active round"))?;
        
        // Extract vote
        let vote = match message.payload {
            MessagePayload::Vote(v) => v,
            _ => return Err(anyhow!("Invalid payload for prevote message")),
        };
        
        debug!(
            "Received prevote from {}: {:?}",
            hex::encode(&message.validator_id[..8]),
            vote.vote_type
        );
        
        // Add prevote to round
        manager.get_round_mut().add_prevote(message.validator_id, vote);
        
        // Try to advance to precommit phase
        manager.try_advance_phase();
        
        Ok(())
    }
    
    /// Process a precommit message
    fn process_precommit(&mut self, message: ConsensusMessage) -> Result<()> {
        let manager = self.round_manager.as_mut()
            .ok_or_else(|| anyhow!("No active round"))?;
        
        // Extract vote
        let vote = match message.payload {
            MessagePayload::Vote(v) => v,
            _ => return Err(anyhow!("Invalid payload for precommit message")),
        };
        
        debug!(
            "Received precommit from {}: {:?}",
            hex::encode(&message.validator_id[..8]),
            vote.vote_type
        );
        
        // Add precommit to round
        manager.get_round_mut().add_precommit(message.validator_id, vote);
        
        // Try to advance to commit phase
        if manager.try_advance_phase() {
            // If we advanced, check if we have consensus
            if manager.get_round().has_precommit_consensus(self.validators.active_count()) {
                manager.mark_consensus_reached();
                info!("Consensus reached for height {}!", self.state.height);
            }
        }
        
        Ok(())
    }
    
    /// Process a commit message
    fn process_commit(&mut self, message: ConsensusMessage) -> Result<()> {
        let manager = self.round_manager.as_ref()
            .ok_or_else(|| anyhow!("No active round"))?;
        
        if !manager.has_consensus() {
            return Err(anyhow!("Commit message received but no consensus"));
        }
        
        info!(
            "Block finalized at height {}",
            self.state.height
        );
        
        Ok(())
    }
    
    /// Create and broadcast proposal (if we are proposer)
    pub fn create_proposal(&self, block_hash: [u8; 32], merkle_root: [u8; 32], tx_count: u32) -> Result<ConsensusMessage> {
        if !self.state.is_proposer {
            return Err(anyhow!("Not the proposer for this round"));
        }
        
        let validator_id = self.our_validator_id
            .ok_or_else(|| anyhow!("Not a validator"))?;
        
        let payload = ProposalPayload {
            block_hash,
            merkle_root,
            transaction_count: tx_count,
            proposer_signature: vec![], // Should sign with validator key
            block_data: None,
        };
        
        Ok(ConsensusMessage {
            message_type: ConsensusMessageType::Proposal,
            height: self.state.height,
            round: self.state.round,
            validator_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            payload: MessagePayload::Proposal(payload),
            signature: vec![], // Should sign message
        })
    }
    
    /// Create prevote message
    pub fn create_prevote(&self, vote_type: VoteType, block_hash: [u8; 32]) -> Result<ConsensusMessage> {
        let validator_id = self.our_validator_id
            .ok_or_else(|| anyhow!("Not a validator"))?;
        
        let vote = ConsensusVote {
            vote_type,
            block_hash,
            reason: None,
        };
        
        Ok(ConsensusMessage {
            message_type: ConsensusMessageType::Prevote,
            height: self.state.height,
            round: self.state.round,
            validator_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            payload: MessagePayload::Vote(vote),
            signature: vec![],
        })
    }
    
    /// Create precommit message
    pub fn create_precommit(&self, vote_type: VoteType, block_hash: [u8; 32]) -> Result<ConsensusMessage> {
        let validator_id = self.our_validator_id
            .ok_or_else(|| anyhow!("Not a validator"))?;
        
        let vote = ConsensusVote {
            vote_type,
            block_hash,
            reason: None,
        };
        
        Ok(ConsensusMessage {
            message_type: ConsensusMessageType::Precommit,
            height: self.state.height,
            round: self.state.round,
            validator_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            payload: MessagePayload::Vote(vote),
            signature: vec![],
        })
    }
    
    /// Tick the consensus engine (check timeouts, advance phases)
    pub fn tick(&mut self) -> Result<Vec<ConsensusAction>> {
        let mut actions = Vec::new();
        
        // Update validator statuses
        self.validators.update_statuses();
        
        if let Some(manager) = self.round_manager.as_mut() {
            // Check for phase timeout
            if manager.is_phase_timeout() {
                warn!(
                    "Phase timeout at height {}, round {}, phase: {:?}",
                    self.state.height,
                    self.state.round,
                    manager.current_phase()
                );
                
                // Try to advance or timeout round
                if !manager.try_advance_phase() {
                    // Round timeout - start new round
                    if self.state.round >= self.max_rounds {
                        error!("Max rounds reached for height {}", self.state.height);
                        actions.push(ConsensusAction::HeightTimeout);
                    } else {
                        actions.push(ConsensusAction::RoundTimeout);
                    }
                }
            }
            
            // Check if we've reached consensus
            if manager.has_consensus() && manager.current_phase() == RoundPhase::Complete {
                if let Some(proposal) = &manager.get_round().proposal {
                    self.state.finalize_block(proposal.block_hash);
                    actions.push(ConsensusAction::BlockFinalized {
                        height: self.state.height,
                        block_hash: proposal.block_hash,
                    });
                }
            }
        }
        
        Ok(actions)
    }
    
    /// Get current consensus state
    pub fn get_state(&self) -> &ConsensusState {
        &self.state
    }
    
    /// Get current round phase
    pub fn current_phase(&self) -> Option<RoundPhase> {
        self.round_manager.as_ref().map(|m| m.current_phase())
    }
    
    /// Rotate to next proposer
    pub fn rotate_proposer(&mut self) {
        self.validators.rotate_proposer();
    }
    
    /// Start a new consensus round
    pub fn start_round(&mut self, height: u64, round: u64) -> Result<()> {
        info!("Starting consensus round {} at height {}", round, height);
        
        self.state.height = height;
        self.state.round = round;
        
        // Determine proposer for this round
        let proposer = self.select_proposer(round as u32)
            .ok_or_else(|| anyhow::anyhow!("No validators available"))?;
        
        // Determine if we are the proposer for this round
        self.state.is_proposer = Some(proposer) == self.our_validator_id;
        
        // Initialize round manager
        self.round_manager = Some(RoundManager::new(
            height,
            round,
            proposer,
            self.state.validator_count,
        ));
        
        Ok(())
    }
    
    /// Finalize block and advance to next height
    pub fn finalize_block(&mut self, block_hash: [u8; 32]) -> Result<()> {
        info!("Finalizing block at height {}", self.state.height);
        
        self.state.finalize_block(block_hash);
        self.round_manager = None; // Clear round manager for next height
        
        Ok(())
    }
    
    /// Select proposer for a given round (deterministic round-robin)
    fn select_proposer(&self, round: u32) -> Option<[u8; 32]> {
        let validators = self.validators.get_active_validators();
        if validators.is_empty() {
            return None;
        }
        
        let index = (round as usize) % validators.len();
        Some(validators[index].validator_id)
    }
}

/// Actions the consensus engine requests
#[derive(Debug, Clone)]
pub enum ConsensusAction {
    /// Block has been finalized
    BlockFinalized {
        height: u64,
        block_hash: [u8; 32],
    },
    /// Current round timed out, start new round
    RoundTimeout,
    /// Max rounds reached for this height
    HeightTimeout,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consensus_engine_creation() {
        let mesh_id = [1u8; 32];
        let genesis_hash = [0u8; 32];
        let validator_id = [2u8; 32];
        
        let engine = MeshConsensusEngine::new(mesh_id, genesis_hash, Some(validator_id));
        
        assert_eq!(engine.mesh_id, mesh_id);
        assert_eq!(engine.our_validator_id, Some(validator_id));
        assert_eq!(engine.state.last_finalized_hash, genesis_hash);
    }
    
    #[test]
    fn test_add_validators() {
        let mut engine = MeshConsensusEngine::new([1u8; 32], [0u8; 32], Some([2u8; 32]));
        
        engine.add_validator(MeshValidator::new([2u8; 32], [3u8; 32], 0));
        engine.add_validator(MeshValidator::new([4u8; 32], [5u8; 32], 0));
        engine.add_validator(MeshValidator::new([6u8; 32], [7u8; 32], 0));
        
        assert_eq!(engine.validators.active_count(), 3);
        assert_eq!(engine.state.validator_count, 3);
    }
}
