//! Mesh Consensus Types
//!
//! Core data structures for mesh consensus protocol

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Consensus message types for mesh BFT protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsensusMessageType {
    /// Proposal message containing a block to validate
    Proposal,
    /// Prevote message (first voting phase)
    Prevote,
    /// Precommit message (commitment phase)
    Precommit,
    /// Commit message (finalization)
    Commit,
}

/// Vote type for consensus messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoteType {
    /// Vote in favor of the proposal
    Approve,
    /// Vote against the proposal
    Reject,
    /// Abstain from voting (validator online but neutral)
    Abstain,
}

/// Consensus message exchanged between validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    /// Type of consensus message
    pub message_type: ConsensusMessageType,
    
    /// Height (block number) this message pertains to
    pub height: u64,
    
    /// Round number within this height
    pub round: u64,
    
    /// Validator ID who sent this message
    pub validator_id: [u8; 32],
    
    /// Timestamp of message creation
    pub timestamp: u64,
    
    /// Message payload (proposal data or vote)
    pub payload: MessagePayload,
    
    /// Signature from validator
    pub signature: Vec<u8>,
}

/// Payload of consensus message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Proposal containing block hash and transactions
    Proposal(ProposalPayload),
    /// Vote on a proposal
    Vote(ConsensusVote),
}

/// Proposal payload containing block to validate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalPayload {
    /// Hash of proposed block
    pub block_hash: [u8; 32],
    
    /// Merkle root of transactions
    pub merkle_root: [u8; 32],
    
    /// Number of transactions in block
    pub transaction_count: u32,
    
    /// Proposer's signature
    pub proposer_signature: Vec<u8>,
    
    /// Optional: Full block data (for small meshes)
    pub block_data: Option<Vec<u8>>,
}

/// Vote on a consensus proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    /// Type of vote (approve/reject/abstain)
    pub vote_type: VoteType,
    
    /// Hash of block being voted on
    pub block_hash: [u8; 32],
    
    /// Optional reason for vote (debugging)
    pub reason: Option<String>,
}

/// Current phase of consensus round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundPhase {
    /// Waiting for proposal
    Propose,
    /// Collecting prevotes
    Prevote,
    /// Collecting precommits
    Precommit,
    /// Finalizing block
    Commit,
    /// Round completed
    Complete,
}

/// Consensus round tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusRound {
    /// Block height this round is for
    pub height: u64,
    
    /// Round number (increments on timeout)
    pub round: u64,
    
    /// Current phase of the round
    pub phase: RoundPhase,
    
    /// Proposer for this round
    pub proposer: [u8; 32],
    
    /// Current proposal (if any)
    pub proposal: Option<ProposalPayload>,
    
    /// Prevotes collected (validator_id -> vote)
    pub prevotes: HashMap<[u8; 32], ConsensusVote>,
    
    /// Precommits collected (validator_id -> vote)
    pub precommits: HashMap<[u8; 32], ConsensusVote>,
    
    /// When this round started
    pub start_time: u64,
    
    /// Whether this round reached consensus
    pub consensus_reached: bool,
}

impl ConsensusRound {
    /// Create new consensus round
    pub fn new(height: u64, round: u64, proposer: [u8; 32]) -> Self {
        Self {
            height,
            round,
            phase: RoundPhase::Propose,
            proposer,
            proposal: None,
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            consensus_reached: false,
        }
    }
    
    /// Check if we have enough prevotes for consensus (67%+)
    pub fn has_prevote_consensus(&self, total_validators: usize) -> bool {
        let approve_count = self.prevotes.values()
            .filter(|v| v.vote_type == VoteType::Approve)
            .count();
        
        let threshold = (total_validators as f64 * super::config::CONSENSUS_THRESHOLD).ceil() as usize;
        approve_count >= threshold
    }
    
    /// Check if we have enough precommits for consensus (67%+)
    pub fn has_precommit_consensus(&self, total_validators: usize) -> bool {
        let approve_count = self.precommits.values()
            .filter(|v| v.vote_type == VoteType::Approve)
            .count();
        
        let threshold = (total_validators as f64 * super::config::CONSENSUS_THRESHOLD).ceil() as usize;
        approve_count >= threshold
    }
    
    /// Add a prevote to this round
    pub fn add_prevote(&mut self, validator_id: [u8; 32], vote: ConsensusVote) {
        self.prevotes.insert(validator_id, vote);
    }
    
    /// Add a precommit to this round
    pub fn add_precommit(&mut self, validator_id: [u8; 32], vote: ConsensusVote) {
        self.precommits.insert(validator_id, vote);
    }
    
    /// Move to next phase
    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            RoundPhase::Propose => RoundPhase::Prevote,
            RoundPhase::Prevote => RoundPhase::Precommit,
            RoundPhase::Precommit => RoundPhase::Commit,
            RoundPhase::Commit => RoundPhase::Complete,
            RoundPhase::Complete => RoundPhase::Complete,
        };
    }
}

/// Overall consensus state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Current block height
    pub height: u64,
    
    /// Current round number
    pub round: u64,
    
    /// Current round details
    pub current_round: ConsensusRound,
    
    /// Last finalized block height
    pub last_finalized_height: u64,
    
    /// Last finalized block hash
    pub last_finalized_hash: [u8; 32],
    
    /// Total number of validators
    pub validator_count: usize,
    
    /// Is this node the current proposer?
    pub is_proposer: bool,
}

impl ConsensusState {
    /// Create new consensus state at genesis
    pub fn new(genesis_hash: [u8; 32]) -> Self {
        Self {
            height: 0,
            round: 0,
            current_round: ConsensusRound::new(0, 0, [0u8; 32]),
            last_finalized_height: 0,
            last_finalized_hash: genesis_hash,
            validator_count: 0,
            is_proposer: false,
        }
    }
    
    /// Start new height
    pub fn start_new_height(&mut self, height: u64, proposer: [u8; 32]) {
        self.height = height;
        self.round = 0;
        self.current_round = ConsensusRound::new(height, 0, proposer);
    }
    
    /// Start new round (on timeout or failure)
    pub fn start_new_round(&mut self, proposer: [u8; 32]) {
        self.round += 1;
        self.current_round = ConsensusRound::new(self.height, self.round, proposer);
    }
    
    /// Finalize block and prepare for next height
    pub fn finalize_block(&mut self, block_hash: [u8; 32]) {
        self.last_finalized_height = self.height;
        self.last_finalized_hash = block_hash;
        self.current_round.consensus_reached = true;
        self.current_round.phase = RoundPhase::Complete;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consensus_round_creation() {
        let proposer = [1u8; 32];
        let round = ConsensusRound::new(1, 0, proposer);
        
        assert_eq!(round.height, 1);
        assert_eq!(round.round, 0);
        assert_eq!(round.phase, RoundPhase::Propose);
        assert_eq!(round.proposer, proposer);
        assert!(!round.consensus_reached);
    }
    
    #[test]
    fn test_prevote_consensus_threshold() {
        let mut round = ConsensusRound::new(1, 0, [1u8; 32]);
        let total_validators = 4;
        
        // Need 3/4 = 67%+ approval
        assert!(!round.has_prevote_consensus(total_validators));
        
        // Add 2 approve votes (50%)
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
        assert!(!round.has_prevote_consensus(total_validators));
        
        // Add 3rd approve vote (75% >= 67%)
        round.add_prevote([4u8; 32], ConsensusVote {
            vote_type: VoteType::Approve,
            block_hash: [0u8; 32],
            reason: None,
        });
        assert!(round.has_prevote_consensus(total_validators));
    }
    
    #[test]
    fn test_phase_advancement() {
        let mut round = ConsensusRound::new(1, 0, [1u8; 32]);
        
        assert_eq!(round.phase, RoundPhase::Propose);
        round.advance_phase();
        assert_eq!(round.phase, RoundPhase::Prevote);
        round.advance_phase();
        assert_eq!(round.phase, RoundPhase::Precommit);
        round.advance_phase();
        assert_eq!(round.phase, RoundPhase::Commit);
        round.advance_phase();
        assert_eq!(round.phase, RoundPhase::Complete);
    }
}
