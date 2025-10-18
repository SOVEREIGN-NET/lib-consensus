//! Round Manager
//!
//! Manages consensus rounds, timeouts, and phase transitions

use super::types::{ConsensusRound, RoundPhase};
use super::config::*;
use std::time::{Duration, Instant};

/// Manages a single consensus round
pub struct RoundManager {
    /// Current round
    round: ConsensusRound,
    
    /// When current phase started
    phase_start: Instant,
    
    /// Total validators in mesh
    total_validators: usize,
}

impl RoundManager {
    /// Create new round manager
    pub fn new(height: u64, round: u64, proposer: [u8; 32], total_validators: usize) -> Self {
        Self {
            round: ConsensusRound::new(height, round, proposer),
            phase_start: Instant::now(),
            total_validators,
        }
    }
    
    /// Get current phase
    pub fn current_phase(&self) -> RoundPhase {
        self.round.phase
    }
    
    /// Get current round
    pub fn get_round(&self) -> &ConsensusRound {
        &self.round
    }
    
    /// Get mutable round
    pub fn get_round_mut(&mut self) -> &mut ConsensusRound {
        &mut self.round
    }
    
    /// Check if current phase has timed out
    pub fn is_phase_timeout(&self) -> bool {
        let timeout = match self.round.phase {
            RoundPhase::Propose => PROPOSAL_TIMEOUT,
            RoundPhase::Prevote => PREVOTE_TIMEOUT,
            RoundPhase::Precommit => PRECOMMIT_TIMEOUT,
            RoundPhase::Commit => Duration::from_millis(200),
            RoundPhase::Complete => return false,
        };
        
        self.phase_start.elapsed() >= timeout
    }
    
    /// Advance to next phase if conditions met
    pub fn try_advance_phase(&mut self) -> bool {
        let can_advance = match self.round.phase {
            RoundPhase::Propose => {
                // Can advance if we have a proposal
                self.round.proposal.is_some()
            }
            RoundPhase::Prevote => {
                // Can advance if we have prevote consensus or timeout
                self.round.has_prevote_consensus(self.total_validators) || self.is_phase_timeout()
            }
            RoundPhase::Precommit => {
                // Can advance if we have precommit consensus or timeout
                self.round.has_precommit_consensus(self.total_validators) || self.is_phase_timeout()
            }
            RoundPhase::Commit => {
                // Commit phase advances after brief finalization period
                self.is_phase_timeout()
            }
            RoundPhase::Complete => {
                // Already complete
                return false;
            }
        };
        
        if can_advance {
            self.round.advance_phase();
            self.phase_start = Instant::now();
            true
        } else {
            false
        }
    }
    
    /// Check if round has reached consensus
    pub fn has_consensus(&self) -> bool {
        self.round.consensus_reached
    }
    
    /// Mark round as having reached consensus
    pub fn mark_consensus_reached(&mut self) {
        self.round.consensus_reached = true;
    }
    
    /// Reset phase timer (useful when receiving late messages)
    pub fn reset_phase_timer(&mut self) {
        self.phase_start = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_consensus::types::{ConsensusVote, VoteType};
    
    #[test]
    fn test_round_manager_creation() {
        let manager = RoundManager::new(1, 0, [1u8; 32], 4);
        
        assert_eq!(manager.current_phase(), RoundPhase::Propose);
        assert!(!manager.has_consensus());
    }
    
    #[test]
    fn test_phase_advancement() {
        let mut manager = RoundManager::new(1, 0, [1u8; 32], 4);
        
        // Can't advance without proposal
        assert!(!manager.try_advance_phase());
        
        // Add proposal
        manager.get_round_mut().proposal = Some(super::super::types::ProposalPayload {
            block_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transaction_count: 5,
            proposer_signature: vec![],
            block_data: None,
        });
        
        // Now can advance to prevote
        assert!(manager.try_advance_phase());
        assert_eq!(manager.current_phase(), RoundPhase::Prevote);
    }
    
    #[test]
    fn test_consensus_threshold() {
        let mut manager = RoundManager::new(1, 0, [1u8; 32], 4);
        
        // Add proposal
        manager.get_round_mut().proposal = Some(super::super::types::ProposalPayload {
            block_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transaction_count: 5,
            proposer_signature: vec![],
            block_data: None,
        });
        manager.try_advance_phase();
        
        // Add prevotes (need 3/4 for 67%+)
        let round = manager.get_round_mut();
        round.add_prevote([2u8; 32], ConsensusVote {
            vote_type: VoteType::Approve,
            block_hash: [0u8; 32],
            reason: None,
        });
        round.add_prevote([3u8; 32], ConsensusVote {
            vote_type: VoteType::Approve,
            block_hash: [0u8; 32],
            reason: None,
        });
        
        // Not enough yet (2/4 = 50%)
        assert!(!round.has_prevote_consensus(4));
        
        // Add one more (3/4 = 75% >= 67%)
        round.add_prevote([4u8; 32], ConsensusVote {
            vote_type: VoteType::Approve,
            block_hash: [0u8; 32],
            reason: None,
        });
        
        assert!(round.has_prevote_consensus(4));
        assert!(manager.try_advance_phase());
        assert_eq!(manager.current_phase(), RoundPhase::Precommit);
    }
}
