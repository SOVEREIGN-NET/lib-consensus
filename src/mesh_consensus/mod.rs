//! Mesh Consensus Module
//!
//! Lightweight BFT consensus engine designed specifically for local mesh networks.
//! Provides fast finality (2-second target) for mesh blockchain blocks while
//! maintaining Byzantine fault tolerance with 67% (2/3) validator agreement.
//!
//! ## Architecture
//!
//! Local mesh networks use a simplified consensus model compared to global validators:
//! - Fast block production (2-second target vs. 10-second global)
//! - Fewer validators (3-10 per mesh vs. 100+ global)
//! - Round-based BFT without PoW (energy efficient for edge devices)
//! - Reputation-based validator selection
//!
//! ## Consensus Flow
//!
//! ```text
//! Round N:
//!   1. PROPOSE: Coordinator proposes block with transactions
//!   2. PREVOTE: Validators vote on proposal validity
//!   3. PRECOMMIT: If 67%+ prevotes, validators precommit
//!   4. COMMIT: If 67%+ precommits, block is finalized
//!   5. NEXT: Move to Round N+1
//! ```
//!
//! ## Integration with LocalMeshBlockchain
//!
//! The MeshConsensusEngine works alongside LocalMeshBlockchain:
//! - LocalMeshBlockchain: State management, UTXO tracking, participant registry
//! - MeshConsensusEngine: Block production, validation, finalization
//! - Together they provide a complete mesh blockchain solution

pub mod engine;
pub mod types;
pub mod validator;
pub mod round;

pub use engine::MeshConsensusEngine;
pub use types::{
    ConsensusMessage, ConsensusMessageType, ConsensusVote, ConsensusRound,
    ProposalPayload, VoteType, RoundPhase, ConsensusState,
};
pub use validator::{MeshValidator, ValidatorSet, ValidatorStatus};
pub use round::RoundManager;

/// Consensus configuration constants
pub mod config {
    use std::time::Duration;
    
    /// Target block time for mesh consensus (2 seconds)
    pub const MESH_BLOCK_TIME: Duration = Duration::from_secs(2);
    
    /// Timeout for proposal phase
    pub const PROPOSAL_TIMEOUT: Duration = Duration::from_millis(500);
    
    /// Timeout for prevote phase
    pub const PREVOTE_TIMEOUT: Duration = Duration::from_millis(500);
    
    /// Timeout for precommit phase
    pub const PRECOMMIT_TIMEOUT: Duration = Duration::from_millis(500);
    
    /// Minimum number of validators required for consensus
    pub const MIN_VALIDATORS: usize = 3;
    
    /// Maximum number of validators per mesh
    pub const MAX_VALIDATORS: usize = 10;
    
    /// Consensus threshold (67% = 2/3 BFT requirement)
    pub const CONSENSUS_THRESHOLD: f64 = 0.67;
    
    /// Maximum rounds before timeout and round increment
    pub const MAX_ROUNDS_PER_HEIGHT: u64 = 10;
    
    /// Validator rotation interval (blocks)
    pub const VALIDATOR_ROTATION_INTERVAL: u64 = 100;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consensus_config() {
        assert_eq!(config::MESH_BLOCK_TIME.as_secs(), 2);
        assert_eq!(config::MIN_VALIDATORS, 3);
        assert_eq!(config::CONSENSUS_THRESHOLD, 0.67);
    }
}
